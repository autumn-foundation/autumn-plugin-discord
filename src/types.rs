//! Typed Discord payloads.
//!
//! Covers what the plugin needs: incoming interactions, slash-command
//! declarations for registration, and interaction responses. Field names
//! follow the Discord API (`snake_case` in JSON).

use serde::{Deserialize, Serialize};

/// Interaction types Discord sends to the webhook.
///
/// Serializes as the numeric type code Discord uses on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InteractionType {
    /// Discord pings the endpoint to verify it.
    Ping = 1,
    /// A user invoked a slash command.
    ApplicationCommand = 2,
    /// A user interacted with a message component.
    MessageComponent = 3,
    /// A user submitted a modal.
    ModalSubmit = 5,
}

impl InteractionType {
    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Ping),
            2 => Some(Self::ApplicationCommand),
            3 => Some(Self::MessageComponent),
            5 => Some(Self::ModalSubmit),
            _ => None,
        }
    }
}

impl Serialize for InteractionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for InteractionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = u8::deserialize(deserializer)?;
        Self::from_code(code).ok_or_else(|| {
            serde::de::Error::custom(format!("unknown interaction type code: {code}"))
        })
    }
}

/// An incoming Discord interaction.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Interaction {
    /// The interaction id (snowflake).
    pub id: String,
    /// The application id (snowflake).
    pub application_id: String,
    /// The interaction type.
    #[serde(rename = "type")]
    pub kind: InteractionType,
    /// Command data for [`InteractionType::ApplicationCommand`].
    pub data: Option<ApplicationCommandData>,
    /// The interaction token, used for follow-up messages.
    pub token: String,
    /// The channel the interaction happened in, when any.
    pub channel_id: Option<String>,
    /// The guild the interaction happened in, when any.
    pub guild_id: Option<String>,
    /// The invoking user or guild member.
    pub member: Option<Member>,
    /// The invoking user in DMs.
    pub user: Option<User>,
}

impl Interaction {
    /// The slash-command name, when this is an application command.
    #[must_use]
    pub fn command_name(&self) -> Option<&str> {
        self.data.as_ref().map(|data| data.name.as_str())
    }
}

/// Data carried by an application-command interaction.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApplicationCommandData {
    /// The invoked command name.
    pub name: String,
    /// The resolved options the user passed.
    #[serde(default)]
    pub options: Vec<CommandOptionValue>,
}

/// One option value passed to a slash command.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandOptionValue {
    /// The option name.
    pub name: String,
    /// The option type code.
    #[serde(rename = "type")]
    pub kind: u8,
    /// The option value (string, integer, boolean, ...).
    pub value: Option<serde_json::Value>,
}

impl CommandOptionValue {
    /// Read the value as a string, when it is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        self.value.as_ref().and_then(serde_json::Value::as_str)
    }

    /// Read the value as an integer, when it is one.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        self.value.as_ref().and_then(serde_json::Value::as_i64)
    }

    /// Read the value as a boolean, when it is one.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        self.value.as_ref().and_then(serde_json::Value::as_bool)
    }
}

/// A guild member invoking the command.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Member {
    /// The underlying user.
    pub user: Option<User>,
    /// The member's guild nickname, when set.
    pub nick: Option<String>,
}

/// A Discord user.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    /// The user id (snowflake).
    pub id: String,
    /// The username.
    pub username: String,
    /// The user's global display name, when set.
    pub global_name: Option<String>,
}

/// Slash-command option types for registration.
///
/// Serializes as the numeric type code Discord expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandOptionType {
    /// A string option.
    String = 3,
    /// An integer option.
    Integer = 4,
    /// A boolean option.
    Boolean = 5,
    /// A user option.
    User = 6,
    /// A channel option.
    Channel = 7,
    /// A role option.
    Role = 8,
    /// A floating-point option.
    Number = 10,
}

impl Serialize for CommandOptionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

/// One declared option of a slash command.
#[derive(Debug, Clone, Serialize)]
pub struct CommandOption {
    /// The option type.
    #[serde(rename = "type")]
    pub kind: CommandOptionType,
    /// The option name (lowercase, no spaces).
    pub name: String,
    /// The option description.
    pub description: String,
    /// Whether the user must supply it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
}

impl CommandOption {
    /// Declare an option.
    #[must_use]
    pub fn new(
        kind: CommandOptionType,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            name: name.into(),
            description: description.into(),
            required: false,
        }
    }

    /// Mark the option as required.
    #[must_use]
    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// A slash command as sent to Discord's registration endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct CommandRegistration {
    /// The command name (lowercase, no spaces).
    pub name: String,
    /// The command description.
    pub description: String,
    /// The command options.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<CommandOption>,
}

/// Interaction response types Discord accepts.
///
/// Serializes as the numeric type code Discord expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InteractionResponseType {
    /// Answer a PING.
    Pong = 1,
    /// Answer a command with a message.
    ChannelMessageWithSource = 4,
    /// Acknowledge the command; the real answer follows later.
    DeferredChannelMessageWithSource = 5,
}

impl Serialize for InteractionResponseType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

/// A message payload sent back to Discord.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MessagePayload {
    /// The message text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Message flags (e.g. 64 for ephemeral).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
}

impl MessagePayload {
    /// A plain-text message.
    #[must_use]
    pub fn content(text: impl Into<String>) -> Self {
        Self {
            content: Some(text.into()),
            flags: None,
        }
    }

    /// Mark the message ephemeral (only the invoker sees it).
    #[must_use]
    pub const fn ephemeral(mut self) -> Self {
        self.flags = Some(64);
        self
    }
}

/// A response to an incoming interaction.
#[derive(Debug, Clone, Serialize)]
pub struct InteractionResponse {
    /// The response type.
    #[serde(rename = "type")]
    pub kind: InteractionResponseType,
    /// The response data, when the type carries a message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<MessagePayload>,
}

impl InteractionResponse {
    /// Answer a PING interaction.
    #[must_use]
    pub const fn pong() -> Self {
        Self {
            kind: InteractionResponseType::Pong,
            data: None,
        }
    }

    /// Answer a command with a message.
    #[must_use]
    pub const fn channel_message(message: MessagePayload) -> Self {
        Self {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(message),
        }
    }

    /// Acknowledge the command now; send the real answer with a follow-up.
    #[must_use]
    pub const fn deferred() -> Self {
        Self {
            kind: InteractionResponseType::DeferredChannelMessageWithSource,
            data: None,
        }
    }
}

/// Skip serializing `false` booleans.
// Serde's `skip_serializing_if` requires `fn(&bool) -> bool`; the reference
// is part of that contract, so the by-value lint does not apply here.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !value
}

#[cfg(test)]
mod tests;
