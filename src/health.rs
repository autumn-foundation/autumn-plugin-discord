//! Readiness check for the Discord plugin.
//!
//! [`DiscordHealth`] implements Autumn's [`HealthIndicator`] trait: it calls
//! `GET /applications/@me` and reports `Up` when the bot token is valid.
//! The plugin installs it under the name `"discord"`, so the token check
//! shows up in the app's `/health` output automatically.

use std::collections::HashMap;
use std::sync::Arc;

use autumn_web::actuator::{HealthCheckOutput, HealthIndicator, HealthStatus};

use crate::client::DiscordClient;

/// Health indicator for the Discord bot token.
pub struct DiscordHealth {
    client: DiscordClient,
}

impl DiscordHealth {
    /// Build the indicator around a REST client.
    #[must_use]
    pub const fn new(client: DiscordClient) -> Self {
        Self { client }
    }

    /// Run the check directly: `Ok` carries the application name.
    ///
    /// # Errors
    ///
    /// Returns the Discord client error when the token check fails.
    pub async fn check_now(&self) -> crate::error::Result<String> {
        self.client.check_token().await.map(|info| info.name)
    }
}

impl HealthIndicator for DiscordHealth {
    fn check(&self) -> futures::future::BoxFuture<'_, HealthCheckOutput> {
        let client = self.client.clone();
        Box::pin(async move {
            match client.check_token().await {
                Ok(info) => {
                    let mut details = HashMap::new();
                    details.insert(
                        "application".to_owned(),
                        serde_json::Value::String(info.name),
                    );
                    HealthCheckOutput {
                        status: HealthStatus::Up,
                        details,
                    }
                }
                Err(err) => {
                    let mut details = HashMap::new();
                    details.insert(
                        "error".to_owned(),
                        serde_json::Value::String(err.to_string()),
                    );
                    HealthCheckOutput {
                        status: HealthStatus::Down,
                        details,
                    }
                }
            }
        })
    }
}

/// Build the shared indicator handle the plugin installs.
#[must_use]
pub fn indicator(client: &DiscordClient) -> Arc<DiscordHealth> {
    Arc::new(DiscordHealth::new(client.clone()))
}

#[cfg(test)]
mod tests;
