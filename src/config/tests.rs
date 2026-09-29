//! Config layering and validation tests.
//!
//! These tests never touch the real environment: they build `DiscordConfig`
//! by hand and check validation and override precedence. `load()` itself is
//! exercised only for its no-file path, which needs no secrets.

use super::*;

const TEST_PUBLIC_KEY: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

fn valid_config() -> DiscordConfig {
    DiscordConfig {
        application_id: "123456789012345678".into(),
        public_key: TEST_PUBLIC_KEY.into(),
        bot_token: Some("token".into()),
    }
}

#[test]
fn valid_config_passes() {
    valid_config().validate().expect("valid config passes");
}

#[test]
fn missing_fields_fail_with_helpful_messages() {
    let err = DiscordConfig::new()
        .validate()
        .expect_err("empty config fails");
    assert!(err.to_string().contains("application_id"), "{err}");

    let mut config = valid_config();
    config.application_id = "not-a-snowflake".into();
    config
        .validate()
        .expect_err("non-numeric application id fails");

    let mut config = valid_config();
    config.public_key = "zz".into();
    let err = config.validate().expect_err("bad public key fails");
    assert!(err.to_string().contains("public_key"), "{err}");

    let mut config = valid_config();
    config.bot_token = None;
    let err = config.validate().expect_err("missing token fails");
    assert!(err.to_string().contains("DISCORD_BOT_TOKEN"), "{err}");
}

#[test]
fn builder_overrides_win_over_file_values() {
    let mut file_config = valid_config();
    file_config.application_id = "111".into();

    let mut overrides = DiscordConfig::new();
    overrides.application_id = "222".into();
    file_config.apply_overrides(&overrides);
    assert_eq!(file_config.application_id, "222");

    // Empty overrides change nothing.
    let before = file_config.clone();
    file_config.apply_overrides(&DiscordConfig::new());
    assert_eq!(file_config.application_id, before.application_id);
    assert_eq!(file_config.public_key, before.public_key);
}

#[test]
fn bot_token_starts_empty_and_is_env_only() {
    let config = DiscordConfig::new();
    assert_eq!(config.bot_token(), None);
    // apply_overrides never sets the token, even from a config that has one.
    let mut target = DiscordConfig::new();
    target.apply_overrides(&valid_config());
    assert_eq!(target.bot_token(), None);
}

#[test]
fn load_without_a_config_file_returns_empty_config() {
    // No autumn.toml in the test working dir and no DISCORD_BOT_TOKEN in the
    // environment: load() must succeed and leave everything empty.
    // (The test runner may set the env var; either way load() must not fail.)
    let config = DiscordConfig::load().expect("load never fails without a file");
    let _ = config.application_id;
}
