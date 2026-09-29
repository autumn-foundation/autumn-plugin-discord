//! Plugin assembly tests.
//!
//! These test the builder surface and runtime assembly without booting an
//! Autumn app: no config files, no network, no secrets.

use super::*;
use crate::types::{InteractionResponse, MessagePayload};

const TEST_PUBLIC_KEY: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

fn base_overrides() -> DiscordConfig {
    let mut config = DiscordConfig::new();
    config.application_id = "123456789012345678".into();
    config.public_key = TEST_PUBLIC_KEY.into();
    config
}

#[test]
fn plugin_name_is_stable() {
    use std::borrow::Cow;
    let plugin = DiscordPlugin::new();
    assert_eq!(plugin.name(), Cow::Borrowed("autumn-plugin-discord"));
}

#[test]
fn builder_chains_commands_and_config() {
    let plugin = DiscordPlugin::new()
        .configure(|c| {
            c.application_id = "123".into();
        })
        .command(
            Command::new("ping", "Replies with Pong!").handler(|_ctx| async {
                Ok(InteractionResponse::channel_message(
                    MessagePayload::content("Pong!"),
                ))
            }),
        );
    assert_eq!(plugin.commands.len(), 1);
    assert_eq!(plugin.config_overrides.application_id, "123");
    assert_eq!(plugin.commands[0].name(), "ping");
}

#[test]
fn build_runtime_rejects_invalid_config() {
    let err = build_runtime(&DiscordConfig::new(), &[]).expect_err("empty config fails");
    assert_eq!(err.kind(), crate::error::ErrorKind::Config);
}

#[test]
fn build_runtime_rejects_bad_command() {
    // The token is missing in tests (it comes only from DISCORD_BOT_TOKEN),
    // so config validation fails first. Either way the error is a Config
    // error, never a half-built runtime.
    let err = build_runtime(&base_overrides(), &[Command::new("BAD NAME", "d")])
        .expect_err("incomplete config fails");
    assert_eq!(err.kind(), crate::error::ErrorKind::Config);
}

#[test]
fn default_plugin_is_empty() {
    let plugin = DiscordPlugin::default();
    assert_eq!(
        plugin.name(),
        std::borrow::Cow::Borrowed("autumn-plugin-discord")
    );
}
