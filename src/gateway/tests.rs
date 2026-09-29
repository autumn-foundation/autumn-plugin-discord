//! Gateway tests against a fake in-process gateway. No Discord involved.

// Tests use `.expect()` for concise setup failures; the
// no-expect rule applies to production code only.
#![allow(clippy::expect_used)]

use super::*;
use std::sync::Mutex as StdMutex;
use tokio::net::TcpListener;

/// Events the fake gateway observed.
#[derive(Default)]
struct Recorder {
    identifies: StdMutex<Vec<String>>,
    heartbeats: StdMutex<u32>,
    events: StdMutex<Vec<String>>,
}

impl EventHandler for Recorder {
    fn on_event(&self, name: &str, _data: &serde_json::Value) {
        self.events
            .lock()
            .expect("test lock holds")
            .push(name.to_owned());
    }
}

/// Run one fake-gateway session: hello, expect identify, count a heartbeat,
/// dispatch one event, then close cleanly.
async fn fake_gateway(listener: TcpListener, recorder: Arc<Recorder>) {
    let (tcp, _) = listener.accept().await.expect("test client connects");
    let mut ws = tokio_tungstenite::accept_async(tcp)
        .await
        .expect("websocket upgrade works");

    // Hello with a short heartbeat interval.
    let hello = json!({"op": 10, "d": {"heartbeat_interval": 50}}).to_string();
    ws.send(Message::Text(hello.into()))
        .await
        .expect("hello sends");

    // Expect Identify carrying our token.
    let identify = read_text(&mut ws).await.expect("identify arrives");
    let value: serde_json::Value = serde_json::from_str(&identify).expect("identify is json");
    assert_eq!(value["op"], 2);
    recorder.identifies.lock().expect("test lock holds").push(
        value["d"]["token"]
            .as_str()
            .expect("identify carries a token")
            .to_owned(),
    );

    // Wait for at least one heartbeat, then dispatch an event.
    let mut saw_heartbeat = false;
    while let Some(message) = ws.next().await {
        let message = message.expect("frame reads");
        if let Message::Text(text) = message {
            let value: serde_json::Value = serde_json::from_str(&text).expect("frame is json");
            if value["op"] == 1 {
                *recorder.heartbeats.lock().expect("test lock holds") += 1;
                saw_heartbeat = true;
                break;
            }
        }
    }
    assert!(saw_heartbeat, "client heartbeats on the hello interval");

    let dispatch = json!({
        "op": 0, "s": 7, "t": "MESSAGE_CREATE",
        "d": {"id": "1", "content": "hello"},
    })
    .to_string();
    ws.send(Message::Text(dispatch.into()))
        .await
        .expect("dispatch sends");

    // Give the client a moment to process, then close cleanly.
    tokio::time::sleep(Duration::from_millis(100)).await;
    ws.close(None).await.expect("close sends");
}

#[tokio::test]
async fn gateway_identifies_heartbeats_and_dispatches() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener binds");
    let url = format!(
        "ws://{}/?v=10&encoding=json",
        listener.local_addr().expect("listener has an addr")
    );
    let recorder = Arc::new(Recorder::default());

    let server_recorder = Arc::clone(&recorder);
    let server = tokio::spawn(async move {
        fake_gateway(listener, server_recorder).await;
    });

    let client = GatewayClient::new("test-token", 0);
    // Clean close from the server ends the run.
    client
        .run_with_url(&url, recorder.as_ref())
        .await
        .expect("gateway run completes on clean close");
    server.await.expect("fake gateway finishes");

    assert_eq!(
        recorder
            .identifies
            .lock()
            .expect("test lock holds")
            .as_slice(),
        ["test-token"]
    );
    assert!(
        *recorder.heartbeats.lock().expect("test lock holds") >= 1,
        "at least one heartbeat"
    );
    assert_eq!(
        recorder.events.lock().expect("test lock holds").as_slice(),
        ["MESSAGE_CREATE"]
    );
}

#[test]
fn identify_payload_shape() {
    let payload = identify_payload("tok", 513);
    let value: serde_json::Value = serde_json::from_str(&payload).expect("valid json");
    assert_eq!(value["op"], 2);
    assert_eq!(value["d"]["token"], "tok");
    assert_eq!(value["d"]["intents"], 513);
}

#[test]
fn heartbeat_payload_before_any_dispatch_is_null() {
    let value: serde_json::Value =
        serde_json::from_str(&heartbeat_payload(-1)).expect("valid json");
    assert_eq!(value["op"], 1);
    assert!(value["d"].is_null());
}

#[test]
fn hello_without_interval_fails() {
    parse_hello(r#"{"op":10,"d":{}}"#).expect_err("missing interval fails");
}
