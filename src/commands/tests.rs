//! Command declaration and dispatch tests.

// Tests use `.expect()` for concise setup failures; the
// no-expect rule applies to production code only.
#![allow(clippy::expect_used)]

use super::*;
use crate::types::{InteractionType, MessagePayload};
use proptest::prelude::*;

fn test_interaction(name: &str) -> Interaction {
    Interaction {
        id: "1".into(),
        application_id: "2".into(),
        kind: InteractionType::ApplicationCommand,
        data: Some(crate::types::ApplicationCommandData {
            name: name.into(),
            options: vec![],
        }),
        token: "tok".into(),
        channel_id: None,
        guild_id: None,
        member: None,
        user: None,
    }
}

fn test_client() -> DiscordClient {
    DiscordClient::new("https://discord.com/api/v10", "token").expect("test client builds")
}

fn test_context(name: &str) -> CommandContext {
    CommandContext {
        interaction: test_interaction(name),
        client: test_client(),
    }
}

#[tokio::test]
async fn dispatch_routes_to_the_right_handler() {
    let mut registry = CommandRegistry::new();
    registry
        .register(
            Command::new("ping", "Replies with Pong!").handler(|_ctx| async {
                Ok(InteractionResponse::channel_message(
                    MessagePayload::content("Pong!"),
                ))
            }),
        )
        .expect("valid command registers");
    registry
        .register(
            Command::new("echo", "Echoes back").handler(|ctx: CommandContext| async move {
                let name = ctx.command_name().unwrap_or("?").to_owned();
                Ok(InteractionResponse::channel_message(
                    MessagePayload::content(format!("echo:{name}")),
                ))
            }),
        )
        .expect("valid command registers");

    let response = registry
        .dispatch(test_context("ping"))
        .await
        .expect("ping dispatches");
    let json = serde_json::to_value(&response).expect("serializes");
    assert_eq!(json["data"]["content"], "Pong!");

    let response = registry
        .dispatch(test_context("echo"))
        .await
        .expect("echo dispatches");
    let json = serde_json::to_value(&response).expect("serializes");
    assert_eq!(json["data"]["content"], "echo:echo");
}

#[tokio::test]
async fn dispatch_unknown_command_fails() {
    let registry = CommandRegistry::new();
    let err = registry
        .dispatch(test_context("nope"))
        .await
        .expect_err("unknown command fails");
    let debug = format!("{err:?}");
    assert!(debug.contains("nope"), "unexpected: {debug}");
}

#[test]
fn invalid_names_reject() {
    for bad in [
        "",
        "UPPER",
        "has space",
        "way-too-long-command-name-over-32-chars",
    ] {
        Command::new(bad, "desc")
            .validate()
            .expect_err(&format!("{bad:?} must reject"));
    }
    Command::new("ok-name_1", "desc")
        .validate()
        .expect("valid name passes");
    Command::new("x", "")
        .validate()
        .expect_err("empty description rejects");
}

#[test]
fn registrations_are_sorted_and_shaped() {
    let mut registry = CommandRegistry::new();
    for name in ["zebra", "apple", "mango"] {
        registry
            .register(
                Command::new(name, "d").handler(|_| async { Ok(InteractionResponse::pong()) }),
            )
            .expect("registers");
    }
    let registrations = registry.registrations();
    let names: Vec<&str> = registrations.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["apple", "mango", "zebra"]);
    assert_eq!(registry.command_names(), ["apple", "mango", "zebra"]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn valid_names_always_register(name in "[a-z0-9_-]{1,32}") {
        let mut registry = CommandRegistry::new();
        registry.register(
            Command::new(name.clone(), "d").handler(|_| async {
                Ok(InteractionResponse::pong())
            }),
        ).expect("generated name registers");
        prop_assert!(registry.command_names().contains(&name.as_str()));
    }
}
