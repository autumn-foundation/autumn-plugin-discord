//! Ed25519 signature verification for Discord interaction webhooks.
//!
//! Discord signs every interaction request. The signature covers the
//! concatenation `timestamp + raw_body`, hex-encoded, and travels in the
//! `X-Signature-Ed25519` header; the timestamp travels in
//! `X-Signature-Timestamp`. The public key is the application's Ed25519 key
//! from the developer portal.
//!
//! [`verify_interaction`] checks both headers and the signature. It fails
//! closed: anything missing or malformed is an [`Error::Signature`].

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::error::{Error, Result};

/// Header carrying the hex-encoded Ed25519 signature.
pub const SIGNATURE_HEADER: &str = "x-signature-ed25519";
/// Header carrying the request timestamp Discord signed.
pub const TIMESTAMP_HEADER: &str = "x-signature-timestamp";

/// Parse a 64-char hex application public key into a [`VerifyingKey`].
pub fn public_key_from_hex(hex_key: &str) -> Result<VerifyingKey> {
    let bytes = hex::decode(hex_key.trim())
        .map_err(|_| Error::Signature("public key is not valid hex".to_owned()))?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| Error::Signature("public key must decode to 32 bytes".to_owned()))?;
    VerifyingKey::from_bytes(&array)
        .map_err(|_| Error::Signature("public key is not a valid Ed25519 key".to_owned()))
}

/// Verify a Discord interaction request.
///
/// `signature_hex` and `timestamp` are the raw header values, `body` is the
/// exact request body bytes Discord sent. The signed message is
/// `timestamp + body`.
pub fn verify_interaction(
    public_key: &VerifyingKey,
    signature_hex: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
) -> Result<()> {
    let signature_hex = signature_hex
        .ok_or_else(|| Error::Signature(format!("missing {SIGNATURE_HEADER} header")))?;
    let timestamp =
        timestamp.ok_or_else(|| Error::Signature(format!("missing {TIMESTAMP_HEADER} header")))?;

    let signature_bytes = hex::decode(signature_hex.trim())
        .map_err(|_| Error::Signature("signature header is not valid hex".to_owned()))?;
    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| Error::Signature("signature must decode to 64 bytes".to_owned()))?;
    let signature = Signature::from_bytes(&signature_array);

    let mut message = Vec::with_capacity(timestamp.len() + body.len());
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(body);

    public_key
        .verify(&message, &signature)
        .map_err(|_| Error::Signature("signature verification failed".to_owned()))
}

#[cfg(test)]
mod tests;
