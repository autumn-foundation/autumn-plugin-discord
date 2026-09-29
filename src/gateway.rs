//! Minimal Discord gateway client.
//!
//! This is a real websocket client, not a stub: it connects to the gateway,
//! answers `Hello` with `Identify`, heartbeats on the negotiated interval,
//! and dispatches events to an [`EventHandler`].
//!
//! What it deliberately does not do: session resume. When the connection
//! drops it reconnects and identifies a fresh session. Long-lived bots that
//! need resume should bring their own gateway or extend this one.
//!
//! The plugin does not start the gateway on its own — spawn it from your own
//! startup hook:
//!
//! ```ignore
//! use autumn_plugin_discord::GatewayClient;
//!
//! let gateway = GatewayClient::new(token, 0);
//! tokio::spawn(async move {
//!     gateway.run(&MyHandler).await;
//! });
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::error::{Error, Result};

/// Discord gateway endpoint.
pub const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

/// Pause before reconnecting after a dropped connection.
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

/// Receives dispatched gateway events.
///
/// Called synchronously on the read loop; spawn work instead of blocking.
pub trait EventHandler: Send + Sync {
    /// Handle a dispatched event, e.g. `MESSAGE_CREATE`.
    fn on_event(&self, name: &str, data: &serde_json::Value);
}

/// A minimal Discord gateway client.
pub struct GatewayClient {
    token: String,
    intents: u32,
}

impl GatewayClient {
    /// Build a client for one bot token and a gateway intent bitmask.
    #[must_use]
    pub fn new(token: impl Into<String>, intents: u32) -> Self {
        Self {
            token: token.into(),
            intents,
        }
    }

    /// Connect and run forever, reconnecting with a fresh session on drops.
    ///
    /// Only returns when the gateway closes the connection cleanly.
    pub async fn run<H: EventHandler>(&self, handler: &H) -> Result<()> {
        self.run_with_url(GATEWAY_URL, handler).await
    }

    /// [`run`](Self::run) against a custom gateway URL (tests, proxies).
    pub async fn run_with_url<H: EventHandler>(&self, url: &str, handler: &H) -> Result<()> {
        loop {
            let closed = self.connect_once(url, handler).await?;
            if closed {
                return Ok(());
            }
            tracing::warn!("discord gateway dropped; reconnecting");
            tokio::time::sleep(RECONNECT_DELAY).await;
        }
    }

    /// One connection lifetime. Returns `Ok(true)` on a clean close,
    /// `Ok(false)` when the caller should reconnect.
    async fn connect_once<H: EventHandler>(&self, url: &str, handler: &H) -> Result<bool> {
        let (stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|err| Error::Gateway(format!("connect failed: {err}")))?;
        let (sink, mut stream) = stream.split();
        let sink = Arc::new(Mutex::new(sink));
        let seq = Arc::new(AtomicI64::new(-1));

        // The first message must be Hello with the heartbeat interval.
        let hello = read_text(&mut stream).await?;
        let interval_ms = parse_hello(&hello)?;

        // Identify.
        {
            let mut guard = sink.lock().await;
            guard
                .send(Message::Text(
                    identify_payload(&self.token, self.intents).into(),
                ))
                .await
                .map_err(|err| Error::Gateway(format!("identify failed: {err}")))?;
        }

        // Heartbeat on the negotiated interval.
        let heartbeat_sink = Arc::clone(&sink);
        let heartbeat_seq = Arc::clone(&seq);
        let heartbeat_task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));
            // The first tick fires immediately; Discord tolerates it.
            loop {
                ticker.tick().await;
                let payload = heartbeat_payload(heartbeat_seq.load(Ordering::SeqCst));
                let mut guard = heartbeat_sink.lock().await;
                if guard.send(Message::Text(payload.into())).await.is_err() {
                    break;
                }
            }
        });

        // Read loop.
        let mut reconnect = true;
        while let Some(message) = stream.next().await {
            let message = message.map_err(|err| Error::Gateway(format!("read failed: {err}")))?;
            match message {
                Message::Text(text) => {
                    let value: serde_json::Value = serde_json::from_str(&text)
                        .map_err(|err| Error::Gateway(format!("bad gateway frame: {err}")))?;
                    let op = value.get("op").and_then(serde_json::Value::as_u64);
                    match op {
                        Some(0) => {
                            if let Some(s) = value.get("s").and_then(serde_json::Value::as_i64) {
                                seq.store(s, Ordering::SeqCst);
                            }
                            if let (Some(name), Some(data)) = (
                                value.get("t").and_then(serde_json::Value::as_str),
                                value.get("d"),
                            ) {
                                handler.on_event(name, data);
                            }
                        }
                        // 7 = Reconnect, 9 = Invalid session: start over.
                        Some(7) | Some(9) => break,
                        // 1 = heartbeat request: answer at once.
                        Some(1) => {
                            let payload = heartbeat_payload(seq.load(Ordering::SeqCst));
                            let mut guard = sink.lock().await;
                            if guard.send(Message::Text(payload.into())).await.is_err() {
                                break;
                            }
                        }
                        // 10 = hello, 11 = heartbeat ack: nothing to do.
                        _ => {}
                    }
                }
                Message::Close(_) => {
                    reconnect = false;
                    break;
                }
                _ => {}
            }
        }

        heartbeat_task.abort();
        drop(sink);
        Ok(!reconnect)
    }
}

/// Read the next text frame from the gateway.
async fn read_text<S>(stream: &mut S) -> Result<String>
where
    S: StreamExt<Item = std::result::Result<Message, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    while let Some(message) = stream.next().await {
        let message = message.map_err(|err| Error::Gateway(format!("read failed: {err}")))?;
        if let Message::Text(text) = message {
            return Ok(text.to_string());
        }
    }
    Err(Error::Gateway("gateway closed before hello".to_owned()))
}

/// Pull the heartbeat interval out of a Hello frame.
fn parse_hello(hello: &str) -> Result<u64> {
    let value: serde_json::Value =
        serde_json::from_str(hello).map_err(|err| Error::Gateway(format!("bad hello: {err}")))?;
    value
        .get("d")
        .and_then(|d| d.get("heartbeat_interval"))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| Error::Gateway("hello carries no heartbeat_interval".to_owned()))
}

/// Build the Identify payload (op 2).
fn identify_payload(token: &str, intents: u32) -> String {
    json!({
        "op": 2,
        "d": {
            "token": token,
            "intents": intents,
            "properties": {
                "os": "linux",
                "browser": "autumn-plugin-discord",
                "device": "autumn-plugin-discord",
            },
        },
    })
    .to_string()
}

/// Build a Heartbeat payload (op 1). `seq` is -1 before any dispatch.
fn heartbeat_payload(seq: i64) -> String {
    let data = if seq < 0 {
        serde_json::Value::Null
    } else {
        serde_json::Value::from(seq)
    };
    json!({ "op": 1, "d": data }).to_string()
}

#[cfg(test)]
mod tests;
