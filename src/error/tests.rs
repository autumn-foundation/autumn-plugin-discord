//! Error kind mapping tests.

// Tests use `.expect()` for concise setup failures; the
// no-expect rule applies to production code only.
#![allow(clippy::expect_used)]

use super::*;

#[test]
fn kind_round_trips() {
    assert_eq!(Error::Config("x".into()).kind(), ErrorKind::Config);
    assert_eq!(Error::Signature("x".into()).kind(), ErrorKind::Signature);
    assert_eq!(
        Error::DiscordApi {
            status: 429,
            body: "rate limited".into()
        }
        .kind(),
        ErrorKind::DiscordApi
    );
    assert_eq!(Error::Gateway("x".into()).kind(), ErrorKind::Gateway);
    assert_eq!(
        Error::UnknownCommand("nope".into()).kind(),
        ErrorKind::UnknownCommand
    );
    assert_eq!(Error::MissingRuntime.kind(), ErrorKind::MissingRuntime);
}

#[test]
fn status_codes_match_documented_mapping() {
    assert_eq!(Error::Signature("bad".into()).status_code(), 401);
    assert_eq!(Error::UnknownCommand("x".into()).status_code(), 400);
    assert_eq!(Error::MissingRuntime.status_code(), 503);
    assert_eq!(Error::Config("x".into()).status_code(), 500);
    assert_eq!(Error::Gateway("x".into()).status_code(), 500);
}

#[test]
fn to_autumn_error_converts() {
    // Conversion must not panic and must keep the message.
    let autumn = Error::Signature("bad signature".into()).to_autumn_error();
    let debug = format!("{autumn:?}");
    assert!(debug.contains("bad signature"), "unexpected: {debug}");
}

#[test]
fn error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Error>();
}
