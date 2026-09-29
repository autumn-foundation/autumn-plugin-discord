//! Webhook extractor tests: signature verification end to end.
//!
//! These drive [`VerifiedInteraction::from_request`] directly with a test
//! `AppState` — no server, no Discord.

// Tests use `.expect()` for concise setup failures; the
// no-expect rule applies to production code only.
#![allow(clippy::expect_used)]

use super::*;
use crate::verify::public_key_from_hex;
use autumn_web::reexports::axum::body::Body;
use autumn_web::reexports::axum::extract::FromRequest;
use autumn_web::reexports::axum::http::Request;
use ed25519_dalek::Signer as _;
use ed25519_dalek::SigningKey;

const RFC8032_SECRET: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const RFC8032_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

fn test_runtime() -> DiscordRuntime {
    let public_key = public_key_from_hex(RFC8032_PUBLIC).expect("test vector key parses");
    let client = crate::client::DiscordClient::new("https://discord.com/api/v10", "tok")
        .expect("test client builds");
    DiscordRuntime {
        public_key,
        client,
        registry: Arc::new(crate::commands::CommandRegistry::new()),
    }
}

fn test_state() -> AppState {
    let state = AppState::for_test();
    state.extension_or_insert_with(test_runtime);
    state
}

fn sign(timestamp: &str, body: &[u8]) -> String {
    let secret_bytes = hex::decode(RFC8032_SECRET).expect("test secret parses");
    let secret_array: [u8; 32] = secret_bytes.try_into().expect("test secret is 32 bytes");
    let signing = SigningKey::from_bytes(&secret_array);
    let mut message = Vec::new();
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(body);
    hex::encode(signing.sign(&message).to_bytes())
}

fn request_with(body: &[u8], signature: Option<&str>, timestamp: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().uri("/discord/interactions");
    if let Some(signature) = signature {
        builder = builder.header(SIGNATURE_HEADER, signature);
    }
    if let Some(timestamp) = timestamp {
        builder = builder.header(TIMESTAMP_HEADER, timestamp);
    }
    builder
        .body(Body::from(body.to_vec()))
        .expect("test request builds")
}

#[tokio::test]
async fn extractor_accepts_a_valid_ping() {
    let body = br#"{"id":"1","application_id":"2","type":1,"token":"tok"}"#;
    let timestamp = "1727654321";
    let signature = sign(timestamp, body);
    let state = test_state();

    let extracted = VerifiedInteraction::from_request(
        request_with(body, Some(&signature), Some(timestamp)),
        &state,
    )
    .await
    .expect("valid signature extracts");
    assert_eq!(extracted.interaction.kind, InteractionType::Ping);
}

#[tokio::test]
async fn extractor_rejects_a_tampered_body() {
    let timestamp = "1727654321";
    let signature = sign(timestamp, b"original");
    let state = test_state();

    let err = VerifiedInteraction::from_request(
        request_with(b"tampered", Some(&signature), Some(timestamp)),
        &state,
    )
    .await
    .expect_err("tampered body rejects");
    let debug = format!("{err:?}");
    assert!(
        debug.contains("401") || debug.contains("Unauthorized"),
        "got: {debug}"
    );
}

#[tokio::test]
async fn extractor_rejects_missing_headers() {
    let state = test_state();
    VerifiedInteraction::from_request(request_with(b"{}", None, None), &state)
        .await
        .expect_err("missing headers reject");
}

#[tokio::test]
async fn extractor_rejects_malformed_json() {
    let body = b"not json";
    let timestamp = "1727654321";
    let signature = sign(timestamp, body);
    let state = test_state();

    VerifiedInteraction::from_request(
        request_with(body, Some(&signature), Some(timestamp)),
        &state,
    )
    .await
    .expect_err("malformed json rejects");
}

#[tokio::test]
async fn extractor_fails_without_runtime() {
    let state = AppState::for_test();
    let err = VerifiedInteraction::from_request(request_with(b"{}", None, None), &state)
        .await
        .expect_err("missing runtime rejects");
    let debug = format!("{err:?}");
    assert!(
        debug.contains("503") || debug.contains("runtime"),
        "got: {debug}"
    );
}

#[test]
fn webhook_route_is_registered() {
    let routes = webhook_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/interactions");
}
