//! Error types for the Discord plugin.
//!
//! [`Error`] is the plugin's own error. [`ErrorKind`] classifies failures so
//! callers can match on them. [`Error::to_autumn_error`] maps each kind to an
//! [`AutumnError`](autumn_web::AutumnError) with a sensible HTTP status.

use autumn_web::AutumnError;
use thiserror::Error;

/// Classifies a [`Error`] without exposing its payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The `[discord]` config is missing or invalid.
    Config,
    /// The interaction signature header is missing, malformed, or invalid.
    Signature,
    /// A JSON payload failed to parse or serialize.
    Json,
    /// A Discord REST call failed at the transport level.
    Http,
    /// The Discord API answered with a non-success status.
    DiscordApi,
    /// The Discord gateway connection failed.
    Gateway,
    /// A command name has no registered handler.
    UnknownCommand,
    /// The plugin runtime is not installed (the startup hook did not run).
    MissingRuntime,
}

/// The plugin's error type.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The `[discord]` config is missing or invalid.
    #[error("discord config error: {0}")]
    Config(String),

    /// The interaction signature is missing, malformed, or invalid.
    #[error("discord signature error: {0}")]
    Signature(String),

    /// A JSON payload failed to parse or serialize.
    #[error("discord json error: {0}")]
    Json(#[from] serde_json::Error),

    /// A Discord REST call failed at the transport level.
    #[error("discord http error: {0}")]
    Http(#[from] reqwest::Error),

    /// The Discord API answered with a non-success status.
    #[error("discord api error: status {status}: {body}")]
    DiscordApi {
        /// The HTTP status code Discord returned.
        status: u16,
        /// The response body, truncated.
        body: String,
    },

    /// The Discord gateway connection failed.
    #[error("discord gateway error: {0}")]
    Gateway(String),

    /// A command name has no registered handler.
    #[error("unknown discord command: {0}")]
    UnknownCommand(String),

    /// The plugin runtime is not installed (the startup hook did not run).
    #[error("discord runtime not installed")]
    MissingRuntime,
}

impl Error {
    /// Classify this error.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Config(_) => ErrorKind::Config,
            Self::Signature(_) => ErrorKind::Signature,
            Self::Json(_) => ErrorKind::Json,
            Self::Http(_) => ErrorKind::Http,
            Self::DiscordApi { .. } => ErrorKind::DiscordApi,
            Self::Gateway(_) => ErrorKind::Gateway,
            Self::UnknownCommand(_) => ErrorKind::UnknownCommand,
            Self::MissingRuntime => ErrorKind::MissingRuntime,
        }
    }

    /// The HTTP status code this error maps to.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        match self.kind() {
            ErrorKind::Config => 500,
            ErrorKind::Signature => 401,
            ErrorKind::Json => 400,
            // Upstream failures surface as 500: AutumnError has no 502
            // constructor, so `to_autumn_error` uses 500 for these.
            ErrorKind::Http | ErrorKind::DiscordApi | ErrorKind::Gateway => 500,
            ErrorKind::UnknownCommand => 400,
            ErrorKind::MissingRuntime => 503,
        }
    }

    /// Convert into an [`AutumnError`](autumn_web::AutumnError).
    ///
    /// The mapping keeps Discord's own semantics: bad signatures are 401,
    /// a missing runtime is 503, upstream Discord failures are 502.
    #[must_use]
    pub fn to_autumn_error(&self) -> AutumnError {
        let message = self.to_string();
        match self.kind() {
            ErrorKind::Signature => AutumnError::unauthorized_msg(message),
            ErrorKind::Json | ErrorKind::UnknownCommand => AutumnError::bad_request_msg(message),
            ErrorKind::MissingRuntime => AutumnError::service_unavailable_msg(message),
            ErrorKind::Config | ErrorKind::Http | ErrorKind::DiscordApi | ErrorKind::Gateway => {
                AutumnError::internal_server_error_msg(message)
            }
        }
    }
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests;
