//! `autumn-plugin-discord`: a Discord bot framework for Autumn.
//!
//! Declare slash commands as data, mount the signed interactions webhook,
//! and answer with typed responses:
//!
//! ```ignore
//! use autumn_plugin_discord::{Command, DiscordPlugin, InteractionResponse, MessagePayload};
//! use autumn_web::prelude::*;
//!
//! async fn ping(ctx: autumn_plugin_discord::CommandContext) -> AutumnResult<InteractionResponse> {
//!     Ok(InteractionResponse::channel_message(MessagePayload::content("Pong!")))
//! }
//!
//! #[autumn_web::main]
//! async fn main() {
//!     autumn_web::app()
//!         .plugin(DiscordPlugin::new().command(Command::new("ping", "Replies with Pong!").handler(ping)))
//!         .run()
//!         .await;
//! }
//! ```
//!
//! ## Configuration
//!
//! ```toml
//! [discord]
//! application_id = "123456789012345678"
//! public_key = "<64 hex chars from the developer portal>"
//! ```
//!
//! The bot token comes only from the `DISCORD_BOT_TOKEN` environment
//! variable — never from the config file.
//!
//! ## Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`config`] | Layered `[discord]` config: file, env, builder overrides |
//! | [`commands`] | Slash-command declarations and dispatch |
//! | [`interactions`] | The signed webhook route and the [`interactions::VerifiedInteraction`] extractor |
//! | [`verify`] | Ed25519 signature verification |
//! | [`client`] | Discord REST client (register commands, follow-ups) |
//! | [`gateway`] | Minimal gateway client (identify, heartbeat, dispatch) |
//! | [`health`] | `GET /applications/@me` readiness check |
//! | [`plugin`] | The [`plugin::DiscordPlugin`] |

#![forbid(unsafe_code)]

pub mod client;
pub mod commands;
pub mod config;
pub mod error;
#[cfg(feature = "gateway")]
pub mod gateway;
pub mod health;
pub mod interactions;
pub mod plugin;
pub mod types;
pub mod verify;

// ── Re-exports ─────────────────────────────────────────────────────
pub use client::{API_BASE, ApplicationInfo, DiscordClient};
pub use commands::{Command, CommandContext, CommandHandler, CommandRegistry, IntoCommandHandler};
pub use config::DiscordConfig;
pub use error::{Error, ErrorKind, Result};
#[cfg(feature = "gateway")]
pub use gateway::{EventHandler, GATEWAY_URL, GatewayClient};
pub use health::{DiscordHealth, indicator as health_indicator};
pub use interactions::VerifiedInteraction;
pub use plugin::{DiscordPlugin, DiscordRuntime};
pub use types::{
    ApplicationCommandData, CommandOption, CommandOptionType, CommandOptionValue,
    CommandRegistration, Interaction, InteractionResponse, InteractionResponseType,
    InteractionType, Member, MessagePayload, User,
};
pub use verify::{SIGNATURE_HEADER, TIMESTAMP_HEADER, public_key_from_hex, verify_interaction};
