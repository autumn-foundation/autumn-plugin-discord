//! Signature verification tests, including an RFC 8032 test vector.

// Tests use `.expect()` for concise setup failures; the
// no-expect rule applies to production code only.
#![allow(clippy::expect_used)]

use super::*;
use ed25519_dalek::{Signer, SigningKey};

// RFC 8032, section 7.1, TEST 1 (empty message).
const RFC8032_SECRET: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const RFC8032_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const RFC8032_SIGNATURE: &str = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

fn rfc8032_key() -> VerifyingKey {
    public_key_from_hex(RFC8032_PUBLIC).expect("test vector key parses")
}

fn rfc8032_sign(message: &[u8]) -> String {
    let secret_bytes = hex::decode(RFC8032_SECRET).expect("test vector secret parses");
    let secret_array: [u8; 32] = secret_bytes
        .try_into()
        .expect("test vector secret is 32 bytes");
    let signing = SigningKey::from_bytes(&secret_array);
    hex::encode(signing.sign(message).to_bytes())
}

#[test]
fn rfc8032_vector_verifies() {
    // Empty message: the published signature must verify.
    verify_interaction(&rfc8032_key(), Some(RFC8032_SIGNATURE), Some(""), b"")
        .expect("RFC 8032 vector verifies");
}

#[test]
fn round_trip_with_timestamp_prefix() {
    let body = br#"{"type":1}"#;
    let timestamp = "1727654321";
    let mut message = Vec::new();
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(body);
    let signature = rfc8032_sign(&message);

    verify_interaction(&rfc8032_key(), Some(&signature), Some(timestamp), body)
        .expect("freshly signed payload verifies");
}

#[test]
fn tampered_body_fails() {
    let timestamp = "1727654321";
    let signature = rfc8032_sign(b"1727654321original");
    let err = verify_interaction(
        &rfc8032_key(),
        Some(&signature),
        Some(timestamp),
        b"tampered",
    )
    .expect_err("tampered body must fail");
    assert_eq!(err.kind(), crate::error::ErrorKind::Signature);
}

#[test]
fn wrong_key_fails() {
    // RFC 8032, section 7.1, TEST 2 public key: valid, but not the signer.
    let other =
        public_key_from_hex("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c")
            .expect("second key parses");
    let signature = rfc8032_sign(b"hello");
    verify_interaction(&other, Some(&signature), Some(""), b"hello")
        .expect_err("wrong key must fail");
}

#[test]
fn missing_headers_fail_closed() {
    let key = rfc8032_key();
    verify_interaction(&key, None, Some("t"), b"body").expect_err("missing signature fails");
    verify_interaction(&key, Some("aa"), None, b"body").expect_err("missing timestamp fails");
}

#[test]
fn malformed_signature_fails() {
    let key = rfc8032_key();
    verify_interaction(&key, Some("not-hex!!"), Some("t"), b"body")
        .expect_err("non-hex signature fails");
    verify_interaction(&key, Some("aa"), Some("t"), b"body").expect_err("short signature fails");
}

#[test]
fn malformed_public_key_fails() {
    public_key_from_hex("not-hex").expect_err("non-hex key fails");
    public_key_from_hex("aa").expect_err("short key fails");
    // 0x02 repeated is not on the curve: decompression finds no x for
    // this y, so `from_bytes` rejects it.
    public_key_from_hex(&"02".repeat(32)).expect_err("non-point key fails");
}

#[test]
fn empty_signature_header_fails() {
    let key = rfc8032_key();
    verify_interaction(&key, Some(""), Some("t"), b"body").expect_err("empty signature fails");
}
