//! The Autumn plugin: [`DiscordPlugin`] and the shared [`DiscordRuntime`].
//!
//! Install it with one line:
//!
//! ```ignore
//! use autumn_plugin_discord::{Command, DiscordPlugin};
//! use autumn_web::prelude::*;
//!
//! #[autumn_web::main]
//! async fn main() {
//!     autumn_web::app()
//!         .plugin(
//!             DiscordPlugin::new()
//!                 .command(Command::new("ping", "Replies with Pong!").handler(ping)),
//!         )
//!         .run()
//!         .await;
//! }
//! ```
//!
//! The plugin declares the `[discord]` config section (so strict config
//! validation accepts it), mounts `POST /discord/interactions`, resolves the
//! config on startup (file, then environment, then builder overrides), and
//! registers a `"discord"` health indicator backed by `GET /applications/@me`.

use std::borrow::Cow;
use std::sync::Arc;

use autumn_web::AppState;
use autumn_web::actuator::IndicatorGroup;
use autumn_web::app::AppBuilder;
use autumn_web::plugin::Plugin;
use autumn_web::plugin_contract::PluginContract;
use autumn_web::reexports::axum::Router;
use autumn_web::route_listing::RouteInfo;
use ed25519_dalek::VerifyingKey;

use crate::client::{API_BASE, DiscordClient};
use crate::commands::{Command, CommandRegistry};
use crate::config::DiscordConfig;
use crate::error::Error;
use crate::health;
use crate::verify::public_key_from_hex;

/// The `autumn-web` range this plugin supports, as a Cargo version
/// requirement.
///
/// The crate versions independently of `autumn-web`, so the range is a
/// literal (not `lockstep_range`). Keep it equal to the `autumn-web`
/// requirement in `Cargo.toml`.
const SUPPORTED_AUTUMN_WEB: &str = "0.8";

/// Runtime state shared by the webhook route and handlers.
///
/// Installed into [`AppState`] extensions by the plugin's startup hook.
/// Handlers reach it through
/// [`VerifiedInteraction`](crate::interactions::VerifiedInteraction).
pub struct DiscordRuntime {
    /// The application's Ed25519 public key, for signature verification.
    pub public_key: VerifyingKey,
    /// The REST client, bound to the bot token.
    pub client: DiscordClient,
    /// The registered slash commands and their handlers.
    pub registry: Arc<CommandRegistry>,
}

/// Debug shows the command count only; the key and token stay out of logs.
impl std::fmt::Debug for DiscordRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordRuntime")
            .field("public_key", &self.public_key)
            .field("client", &self.client)
            .field("commands", &self.registry.command_names().len())
            .finish_non_exhaustive()
    }
}

/// A Discord bot as an Autumn plugin.
pub struct DiscordPlugin {
    config_overrides: DiscordConfig,
    commands: Vec<Command>,
}

impl DiscordPlugin {
    /// Create the plugin. Add commands with [`command`](Self::command) and
    /// config overrides with [`configure`](Self::configure).
    #[must_use]
    pub fn new() -> Self {
        Self {
            config_overrides: DiscordConfig::new(),
            commands: Vec::new(),
        }
    }

    /// Override config values in code. Values set here win over `autumn.toml`.
    ///
    /// The bot token cannot be set here — it comes only from the
    /// `DISCORD_BOT_TOKEN` environment variable.
    #[must_use]
    pub fn configure(mut self, configure: impl FnOnce(&mut DiscordConfig)) -> Self {
        configure(&mut self.config_overrides);
        self
    }

    /// Declare a slash command served by this bot.
    #[must_use]
    pub fn command(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }
}

impl Default for DiscordPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DiscordPlugin {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("autumn-plugin-discord")
    }

    /// Declare the supported `autumn-web` range. An app that mounts this
    /// plugin into an incompatible framework fails at registration with a
    /// diagnostic that names both versions.
    fn contract(&self) -> Option<PluginContract> {
        Some(
            PluginContract::new(env!("CARGO_PKG_NAME"))
                .plugin_version(env!("CARGO_PKG_VERSION"))
                .autumn_web(SUPPORTED_AUTUMN_WEB),
        )
    }

    fn build(self, app: AppBuilder) -> AppBuilder {
        let config_overrides = self.config_overrides;
        let commands = Arc::new(self.commands);
        let (router, infos) = webhook_mount();

        app.config_section(crate::config::CONFIG_SECTION)
            .on_startup(move |state: AppState| {
                let config_overrides = config_overrides.clone();
                let commands = Arc::clone(&commands);
                async move {
                    let runtime = build_runtime(&config_overrides, &commands)
                        .map_err(|err| err.to_autumn_error())?;
                    let indicator = health::indicator(&runtime.client);
                    // A duplicate name only happens when the plugin is
                    // installed twice; the framework already warns there.
                    let _ = state.health_indicator_registry().register(
                        "discord",
                        IndicatorGroup::Readiness,
                        indicator,
                    );
                    tracing::info!(
                        commands = runtime.registry.command_names().len(),
                        "discord plugin ready"
                    );
                    state.extension_or_insert_with(|| runtime);
                    Ok(())
                }
            })
            .nest("/discord", router)
            .declare_plugin_routes(infos)
    }
}

/// Resolve config, validate it, and build the runtime.
fn build_runtime(
    config_overrides: &DiscordConfig,
    commands: &[Command],
) -> crate::error::Result<DiscordRuntime> {
    let mut config = DiscordConfig::load()?;
    config.apply_overrides(config_overrides);
    config.validate()?;

    let public_key = public_key_from_hex(&config.public_key)?;
    let token = config
        .bot_token()
        .ok_or_else(|| Error::Config("DISCORD_BOT_TOKEN is not set".to_owned()))?;
    let client =
        DiscordClient::new(API_BASE, token)?.with_application_id(config.application_id.clone());

    let mut registry = CommandRegistry::new();
    for command in commands {
        registry.register(command.clone())?;
    }

    Ok(DiscordRuntime {
        public_key,
        client,
        registry: Arc::new(registry),
    })
}

/// The plugin's routes mounted under `/discord`.
///
/// Returns the Axum router for [`AppBuilder::nest`] plus the route metadata
/// for [`AppBuilder::declare_plugin_routes`], so `autumn routes audit` sees
/// the mount as enumerable instead of opaque.
fn webhook_mount() -> (Router<AppState>, Vec<RouteInfo>) {
    let mut router = Router::<AppState>::new();
    let mut infos = Vec::new();
    for route in crate::interactions::webhook_routes() {
        infos.push(RouteInfo {
            method: route.method.to_string(),
            path: format!("/discord{}", route.path),
            handler: route.name.to_owned(),
            module: Some(module_path!().to_owned()),
            ..Default::default()
        });
        router = router.route(route.path, route.handler);
    }
    (router, infos)
}

#[cfg(test)]
mod tests;
