//! Payload parsing tests.

// Tests use `.expect()` for concise setup failures; the
// no-expect rule applies to production code only.
#![allow(clippy::expect_used)]

use super::*;

// A real-shaped application-command interaction, as Discord sends it.
const PING_BODY: &str = r#"{"id":"1","application_id":"2","type":1,"token":"tok"}"#;

const COMMAND_BODY: &str = r#"{
  "id": "123",
  "application_id": "456",
  "type": 2,
  "token": "interaction-token",
  "channel_id": "789",
  "guild_id": "321",
  "data": {
    "name": "ping",
    "options": [
      {"name": "target", "type": 3, "value": "world"},
      {"name": "count", "type": 4, "value": 3},
      {"name": "loud", "type": 5, "value": true}
    ]
  },
  "user": {"id": "999", "username": "mark", "global_name": "Mark"}
}"#;

#[test]
fn parses_ping_interaction() {
    let interaction: Interaction = serde_json::from_str(PING_BODY).expect("ping parses");
    assert_eq!(interaction.kind, InteractionType::Ping);
    assert_eq!(interaction.command_name(), None);
}

#[test]
fn parses_command_interaction() {
    let interaction: Interaction = serde_json::from_str(COMMAND_BODY).expect("command parses");
    assert_eq!(interaction.kind, InteractionType::ApplicationCommand);
    assert_eq!(interaction.command_name(), Some("ping"));
    assert_eq!(interaction.token, "interaction-token");

    let options = &interaction.data.expect("data present").options;
    assert_eq!(options.len(), 3);
    assert_eq!(options[0].as_str(), Some("world"));
    assert_eq!(options[1].as_i64(), Some(3));
    assert_eq!(options[2].as_bool(), Some(true));
    assert_eq!(options[0].as_i64(), None);

    let user = interaction.user.expect("user present");
    assert_eq!(user.username, "mark");
}

#[test]
fn rejects_unknown_interaction_type() {
    let body = r#"{"id":"1","application_id":"2","type":99,"token":"tok"}"#;
    serde_json::from_str::<Interaction>(body).expect_err("unknown type rejects");
}

#[test]
fn response_serializes_to_discord_shape() {
    let response = InteractionResponse::channel_message(MessagePayload::content("Pong!"));
    let json = serde_json::to_value(&response).expect("serializes");
    assert_eq!(json["type"], 4);
    assert_eq!(json["data"]["content"], "Pong!");
    assert!(json.get("data").expect("data").get("flags").is_none());

    let ephemeral = InteractionResponse::channel_message(MessagePayload::content("hi").ephemeral());
    let json = serde_json::to_value(&ephemeral).expect("serializes");
    assert_eq!(json["data"]["flags"], 64);
}

#[test]
fn pong_has_no_data() {
    let json = serde_json::to_value(InteractionResponse::pong()).expect("serializes");
    assert_eq!(json["type"], 1);
    assert!(json.get("data").is_none());
}

#[test]
fn registration_serializes_options() {
    let registration = CommandRegistration {
        name: "greet".into(),
        description: "Say hello".into(),
        options: vec![
            CommandOption::new(CommandOptionType::String, "name", "Who to greet").required(),
        ],
    };
    let json = serde_json::to_value(&registration).expect("serializes");
    assert_eq!(json["name"], "greet");
    assert_eq!(json["options"][0]["type"], 3);
    assert_eq!(json["options"][0]["required"], true);
}
