//! Slash-command declarations and dispatch.
//!
//! Declare commands as data with [`Command`], register them on
//! [`DiscordPlugin`](crate::plugin::DiscordPlugin), and let the interactions
//! webhook route them to handlers. Handlers are plain async functions taking
//! a [`CommandContext`] and returning an [`InteractionResponse`].

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use autumn_web::{AutumnError, AutumnResult};

use crate::client::DiscordClient;
use crate::error::{Error, Result};
use crate::types::{
    CommandOption, CommandRegistration, Interaction, InteractionResponse, MessagePayload,
};

/// The boxed future a command handler returns.
pub type CommandFuture =
    Pin<Box<dyn Future<Output = AutumnResult<InteractionResponse>> + Send + 'static>>;

/// A command handler: takes the context, answers the interaction.
pub type CommandHandler = Arc<dyn Fn(CommandContext) -> CommandFuture + Send + Sync + 'static>;

/// What a command handler receives.
#[derive(Clone)]
pub struct CommandContext {
    /// The incoming interaction.
    pub interaction: Interaction,
    /// A REST client for follow-ups and deferred answers.
    pub client: DiscordClient,
}

impl CommandContext {
    /// The slash-command name that triggered this handler.
    #[must_use]
    pub fn command_name(&self) -> Option<&str> {
        self.interaction.command_name()
    }
}

/// Convert an async function into a [`CommandHandler`].
///
/// Implemented for `Fn(CommandContext) -> Fut` where the future resolves to
/// `AutumnResult<InteractionResponse>`.
pub trait IntoCommandHandler {
    /// Box the function as a handler.
    fn into_handler(self) -> CommandHandler;
}

impl<F, Fut> IntoCommandHandler for F
where
    F: Fn(CommandContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = AutumnResult<InteractionResponse>> + Send + 'static,
{
    fn into_handler(self) -> CommandHandler {
        Arc::new(move |ctx| Box::pin(self(ctx)))
    }
}

/// A declared slash command: its Discord-facing shape plus its handler.
#[derive(Clone)]
pub struct Command {
    name: String,
    description: String,
    options: Vec<CommandOption>,
    handler: CommandHandler,
}

impl Command {
    /// Declare a command with a name and description.
    ///
    /// Names must be 1–32 chars of lowercase letters, digits, `-`, or `_`,
    /// per Discord's rules.
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            options: Vec::new(),
            handler: Arc::new(|_| {
                Box::pin(async {
                    Ok(InteractionResponse::channel_message(
                        MessagePayload::content("no handler registered"),
                    ))
                })
            }),
        }
    }

    /// Add an option to the command declaration.
    #[must_use]
    pub fn option(mut self, option: CommandOption) -> Self {
        self.options.push(option);
        self
    }

    /// Set the handler that answers this command.
    #[must_use]
    pub fn handler(mut self, handler: impl IntoCommandHandler) -> Self {
        self.handler = handler.into_handler();
        self
    }

    /// The command name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Validate the declaration against Discord's naming rules.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] when the name or description breaks
    /// Discord's rules (name: 1-32 chars of lowercase letters, digits,
    /// `-`, `_`; description: 1-100 chars).
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() || self.name.len() > 32 {
            return Err(Error::Config(format!(
                "command name {:?} must be 1-32 chars",
                self.name
            )));
        }
        let valid = self
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
        if !valid {
            return Err(Error::Config(format!(
                "command name {:?} must be lowercase letters, digits, '-', or '_'",
                self.name
            )));
        }
        if self.description.is_empty() || self.description.len() > 100 {
            return Err(Error::Config(format!(
                "command {:?} description must be 1-100 chars",
                self.name
            )));
        }
        Ok(())
    }

    /// The registration payload sent to Discord.
    #[must_use]
    pub fn registration(&self) -> CommandRegistration {
        CommandRegistration {
            name: self.name.clone(),
            description: self.description.clone(),
            options: self.options.clone(),
        }
    }
}

/// Routes incoming command interactions to their handlers.
#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a command. Replaces any command with the same name.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] when the command fails
    /// [`validate`](Command::validate).
    pub fn register(&mut self, command: Command) -> Result<()> {
        command.validate()?;
        self.commands.insert(command.name.clone(), command);
        Ok(())
    }

    /// The command names currently registered.
    #[must_use]
    pub fn command_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.commands.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// The registration payloads for Discord's bulk-overwrite endpoint.
    #[must_use]
    pub fn registrations(&self) -> Vec<CommandRegistration> {
        let mut registrations: Vec<CommandRegistration> =
            self.commands.values().map(Command::registration).collect();
        registrations.sort_by(|a, b| a.name.cmp(&b.name));
        registrations
    }

    /// Dispatch an application-command interaction to its handler.
    ///
    /// Returns [`Error::UnknownCommand`] when no handler is registered for
    /// the command name.
    ///
    /// # Errors
    ///
    /// Returns an [`AutumnError`] when the interaction carries no command
    /// name, when no handler is registered for it, or when the handler
    /// itself fails.
    pub async fn dispatch(&self, ctx: CommandContext) -> AutumnResult<InteractionResponse> {
        let name = ctx
            .command_name()
            .ok_or_else(|| AutumnError::bad_request_msg("interaction carries no command name"))?;
        let command = self
            .commands
            .get(name)
            .ok_or_else(|| Error::UnknownCommand(name.to_owned()).to_autumn_error())?;
        (command.handler)(ctx).await
    }
}

#[cfg(test)]
mod tests;
