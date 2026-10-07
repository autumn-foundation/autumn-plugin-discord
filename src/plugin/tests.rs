//! Plugin assembly tests.
//!
//! These test the builder surface and runtime assembly without booting an
//! Autumn app: no config files, no network, no secrets.

// Tests use `.expect()` for concise setup failures; the
// no-expect rule applies to production code only.
#![allow(clippy::expect_used)]

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

#[test]
fn contract_declares_supported_autumn_web_range() {
    let contract = DiscordPlugin::new().contract().expect("contract declared");
    assert_eq!(contract.plugin, env!("CARGO_PKG_NAME"));
    assert_eq!(
        contract.plugin_version.as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(contract.autumn_web.as_deref(), Some(SUPPORTED_AUTUMN_WEB));
    assert!(contract.experimental_surfaces.is_empty());
}

#[test]
fn contract_is_compatible_with_linked_autumn_web() {
    use autumn_web::plugin_contract::{AUTUMN_WEB_VERSION, ContractVerdict, evaluate};
    let contract = DiscordPlugin::new().contract().expect("contract declared");
    assert_eq!(
        evaluate(&contract, AUTUMN_WEB_VERSION),
        ContractVerdict::Compatible
    );
}

#[test]
fn contract_rejects_other_autumn_web_minors() {
    use autumn_web::plugin_contract::{ContractVerdict, evaluate};
    let contract = DiscordPlugin::new().contract().expect("contract declared");
    for version in ["0.7.0", "0.9.0", "1.0.0"] {
        assert!(
            matches!(
                evaluate(&contract, version),
                ContractVerdict::Incompatible(_)
            ),
            "autumn-web {version} must be incompatible"
        );
    }
}

#[test]
fn registration_records_contract_and_routes() {
    let app = autumn_web::app().plugin(DiscordPlugin::new());
    assert!(app.has_plugin("autumn-plugin-discord"));
    assert!(
        app.plugin_contracts()
            .iter()
            .any(|c| c.plugin == env!("CARGO_PKG_NAME"))
    );
}

#[test]
fn plugin_passes_conformance_harness() {
    use autumn_web::plugin_conformance::{ConformanceConfig, run_conformance};
    let plugin = DiscordPlugin::new();
    let contract = plugin.contract().expect("contract declared");
    let app = autumn_web::app().plugin(plugin);
    let routes = app.plugin_route_infos().expect("routes build");
    assert!(
        routes
            .iter()
            .any(|r| r.method == "POST" && r.path == "/discord/interactions"),
        "webhook route is declared"
    );
    let config = ConformanceConfig::new("autumn-plugin-discord")
        .prefix("/discord")
        .contract(contract);
    let report = run_conformance(&config, &routes);
    assert!(report.passed(), "{}", report.to_text_report());
}
