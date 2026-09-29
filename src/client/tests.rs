//! REST client tests. No network: a local TCP stub stands in for Discord.

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

/// Serve one HTTP response, then stop. Returns the base URL to call.
fn stub_server(status: u16, body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener binds");
    let addr = listener.local_addr().expect("listener has an addr");
    let body = body.to_owned();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accepts one connection");
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        let response = format!(
            "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn check_token_parses_application_info() {
    let base = stub_server(200, r#"{"id":"456","name":"TestBot"}"#);
    let client = DiscordClient::new(base, "tok").expect("client builds");
    let info = client.check_token().await.expect("token check works");
    assert_eq!(info.id, "456");
    assert_eq!(info.name, "TestBot");
}

#[tokio::test]
async fn api_error_surfaces_status_and_body() {
    let base = stub_server(401, r#"{"message":"401: Unauthorized"}"#);
    let client = DiscordClient::new(base, "bad").expect("client builds");
    let err = client.check_token().await.expect_err("401 fails");
    assert_eq!(err.kind(), crate::error::ErrorKind::DiscordApi);
    match err {
        Error::DiscordApi { status, body } => {
            assert_eq!(status, 401);
            assert!(body.contains("Unauthorized"), "body: {body}");
        }
        other => panic!("expected DiscordApi, got: {other:?}"),
    }
}

#[tokio::test]
async fn register_commands_needs_application_id() {
    let client = DiscordClient::new("http://127.0.0.1:9", "tok").expect("client builds");
    let err = client
        .register_commands(&[])
        .await
        .expect_err("missing app id fails");
    assert_eq!(err.kind(), crate::error::ErrorKind::Config);
}

#[tokio::test]
async fn register_commands_round_trips() {
    let base = stub_server(200, r#"[{"id":"1","name":"ping"}]"#);
    let client = DiscordClient::new(base, "tok")
        .expect("client builds")
        .with_application_id("456");
    let commands = client
        .register_commands(&[CommandRegistration {
            name: "ping".into(),
            description: "Pong".into(),
            options: vec![],
        }])
        .await
        .expect("registration works");
    assert_eq!(commands.len(), 1);
}

#[test]
fn empty_token_rejects() {
    DiscordClient::new(API_BASE, "  ").expect_err("empty token rejects");
}
