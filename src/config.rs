//! Layered configuration for the Discord plugin.
//!
//! [`DiscordConfig`] reads the `[discord]` section of `autumn.toml` and
//! applies overrides. The bot token comes only from the `DISCORD_BOT_TOKEN`
//! environment variable — it never appears in the config file.
//!
//! Precedence, highest first:
//!
//! 1. Builder overrides set through
//!    [`DiscordPlugin::configure`](crate::plugin::DiscordPlugin::configure).
//! 2. The `[discord]` section of `autumn.toml` (searched in
//!    `AUTUMN_MANIFEST_DIR`, then the current working directory).
//! 3. Built-in defaults (empty: every field is required).

use std::path::PathBuf;

use crate::error::{Error, Result};

/// Name of the environment variable that carries the bot token.
pub const BOT_TOKEN_ENV_VAR: &str = "DISCORD_BOT_TOKEN";

/// Name of the `autumn.toml` section this plugin reads.
pub const CONFIG_SECTION: &str = "discord";

/// Layered configuration for the Discord plugin.
///
/// Build it with [`DiscordConfig::load`], which reads `autumn.toml` and the
/// environment, or construct it by hand for tests. The `Debug` impl redacts
/// the bot token.
#[derive(Clone, Default)]
pub struct DiscordConfig {
    /// The Discord application id (a numeric snowflake as a string).
    pub application_id: String,
    /// The application public key, as hex (64 chars, 32 bytes).
    pub public_key: String,
    /// The bot token. Set only from `DISCORD_BOT_TOKEN`; never from the file.
    bot_token: Option<String>,
}

impl std::fmt::Debug for DiscordConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordConfig")
            .field("application_id", &self.application_id)
            .field("public_key", &self.public_key)
            .field("bot_token", &self.bot_token.as_ref().map(|_| "<redacted>"))
            .finish_non_exhaustive()
    }
}

impl DiscordConfig {
    /// Create an empty config. Useful with
    /// [`DiscordPlugin::configure`](crate::plugin::DiscordPlugin::configure).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the `[discord]` section of `autumn.toml` and the bot token from
    /// the environment.
    ///
    /// The file is searched in `AUTUMN_MANIFEST_DIR` first, then the current
    /// working directory — the same order the framework's own config loader
    /// uses. A missing file is fine; a missing section is fine; the result
    /// simply stays empty until [`validate`](Self::validate) runs.
    ///
    /// A `bot_token` key inside `[discord]` is ignored on purpose: the token
    /// must come from the environment.
    pub fn load() -> Result<Self> {
        let mut config = Self::new();
        if let Some(table) = read_section_table()? {
            if let Some(value) = table.get("application_id").and_then(|v| v.as_str()) {
                config.application_id = value.to_owned();
            }
            if let Some(value) = table.get("public_key").and_then(|v| v.as_str()) {
                config.public_key = value.to_owned();
            }
        }
        if let Ok(token) = std::env::var(BOT_TOKEN_ENV_VAR) {
            if !token.trim().is_empty() {
                config.bot_token = Some(token);
            }
        }
        Ok(config)
    }

    /// Overlay non-empty builder values on top of this config.
    ///
    /// Builder values win over file values.
    pub fn apply_overrides(&mut self, overrides: &Self) {
        if !overrides.application_id.is_empty() {
            self.application_id.clone_from(&overrides.application_id);
        }
        if !overrides.public_key.is_empty() {
            self.public_key.clone_from(&overrides.public_key);
        }
        // The token travels only through the environment.
    }

    /// The bot token, when the environment provided one.
    #[must_use]
    pub fn bot_token(&self) -> Option<&str> {
        self.bot_token.as_deref()
    }

    /// Check that every required field is present and well-formed.
    ///
    /// Returns the first problem found: a missing application id, a missing
    /// or malformed public key, or a missing bot token.
    pub fn validate(&self) -> Result<()> {
        if self.application_id.trim().is_empty() {
            return Err(Error::Config(
                "discord.application_id is missing; set it in autumn.toml [discord]".to_owned(),
            ));
        }
        if !self
            .application_id
            .trim()
            .chars()
            .all(|c| c.is_ascii_digit())
        {
            return Err(Error::Config(
                "discord.application_id must be a numeric snowflake".to_owned(),
            ));
        }
        let key = self.public_key.trim();
        if key.len() != 64 || hex::decode(key).is_err() {
            return Err(Error::Config(
                "discord.public_key must be 64 hex chars (the Ed25519 application public key)"
                    .to_owned(),
            ));
        }
        match &self.bot_token {
            Some(token) if !token.trim().is_empty() => Ok(()),
            _ => Err(Error::Config(format!(
                "discord bot token is missing; set the {BOT_TOKEN_ENV_VAR} environment variable"
            ))),
        }
    }
}

/// Locate `autumn.toml` (`AUTUMN_MANIFEST_DIR` first, then the CWD) and
/// return the parsed `[discord]` table, when the file and section exist.
fn read_section_table() -> Result<Option<toml::Table>> {
    let path = config_file_path();
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(_) => return Ok(None),
    };
    let table: toml::Table = toml::from_str(&contents)
        .map_err(|err| Error::Config(format!("invalid {}: {err}", path.display())))?;
    Ok(table
        .get(CONFIG_SECTION)
        .and_then(|value| value.as_table())
        .cloned())
}

/// Mirror the framework's config-file discovery order.
fn config_file_path() -> PathBuf {
    if let Ok(manifest_dir) = std::env::var("AUTUMN_MANIFEST_DIR") {
        let candidate = PathBuf::from(manifest_dir).join("autumn.toml");
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from("autumn.toml")
}

#[cfg(test)]
mod tests;
