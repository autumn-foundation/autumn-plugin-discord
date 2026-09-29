//! Discord REST client.
//!
//! [`DiscordClient`] talks to Discord's HTTP API: it registers slash commands,
//! sends follow-up messages for deferred interactions, and checks the bot
//! token with `GET /applications/@me`.
//!
//! The client is cheap to clone and safe to share across handlers.

use std::time::Duration;

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::types::{CommandRegistration, MessagePayload};

/// Discord API base URL.
pub const API_BASE: &str = "https://discord.com/api/v10";

/// Request timeout for REST calls.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The bot's own application record, from `GET /applications/@me`.
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationInfo {
    /// The application id.
    pub id: String,
    /// The application name.
    pub name: String,
}

/// A Discord REST client bound to one bot token.
///
/// Cheap to clone and safe to share across handlers. The `Debug` impl
/// redacts the token.
#[derive(Clone)]
pub struct DiscordClient {
    inner: reqwest::Client,
    base_url: String,
    application_id: String,
    token: String,
}

impl std::fmt::Debug for DiscordClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordClient")
            .field("base_url", &self.base_url)
            .field("application_id", &self.application_id)
            .field("token", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl DiscordClient {
    /// Build a client. The base URL defaults to [`API_BASE`]; pass a custom
    /// one in tests to point at a mock server.
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Result<Self> {
        let token = token.into();
        if token.trim().is_empty() {
            return Err(Error::Config("bot token is empty".to_owned()));
        }
        let inner = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent("autumn-plugin-discord/0.1.0")
            .build()
            .map_err(Error::Http)?;
        Ok(Self {
            inner,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            application_id: String::new(),
            token,
        })
    }

    /// Set the application id used for command registration.
    #[must_use]
    pub fn with_application_id(mut self, application_id: impl Into<String>) -> Self {
        self.application_id = application_id.into();
        self
    }

    /// Check the bot token: `GET /applications/@me`.
    ///
    /// Returns the application record when the token is valid.
    pub async fn check_token(&self) -> Result<ApplicationInfo> {
        let response = self
            .inner
            .get(format!("{}/applications/@me", self.base_url))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(Error::Http)?;
        parse_response(response).await
    }

    /// Bulk-overwrite the global slash commands:
    /// `PUT /applications/{application_id}/commands`.
    pub async fn register_commands(
        &self,
        commands: &[CommandRegistration],
    ) -> Result<Vec<serde_json::Value>> {
        if self.application_id.trim().is_empty() {
            return Err(Error::Config(
                "application_id is missing; cannot register commands".to_owned(),
            ));
        }
        let response = self
            .inner
            .put(format!(
                "{}/applications/{}/commands",
                self.base_url, self.application_id
            ))
            .bearer_auth(&self.token)
            .json(commands)
            .send()
            .await
            .map_err(Error::Http)?;
        parse_response(response).await
    }

    /// Send a follow-up message to a deferred interaction:
    /// `POST /webhooks/{application_id}/{interaction_token}`.
    pub async fn create_followup(
        &self,
        interaction_token: &str,
        message: &MessagePayload,
    ) -> Result<serde_json::Value> {
        if self.application_id.trim().is_empty() {
            return Err(Error::Config(
                "application_id is missing; cannot send follow-ups".to_owned(),
            ));
        }
        let response = self
            .inner
            .post(format!(
                "{}/webhooks/{}/{interaction_token}",
                self.base_url, self.application_id
            ))
            .bearer_auth(&self.token)
            .json(message)
            .send()
            .await
            .map_err(Error::Http)?;
        parse_response(response).await
    }
}

/// Parse a Discord response: success decodes as JSON, failure becomes
/// [`Error::DiscordApi`] with the status and a truncated body.
async fn parse_response<T>(response: reqwest::Response) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();
    if status.is_success() {
        let bytes = response.bytes().await.map_err(Error::Http)?;
        serde_json::from_slice::<T>(&bytes).map_err(Error::Json)
    } else {
        let code = status.as_u16();
        let body = match response.text().await {
            Ok(body) => body,
            Err(_) => String::new(),
        };
        let truncated: String = body.chars().take(500).collect();
        Err(Error::DiscordApi {
            status: code,
            body: truncated,
        })
    }
}

#[cfg(test)]
mod tests;
