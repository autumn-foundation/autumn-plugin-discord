//! A minimal Discord bot: a `/ping` slash command.
//!
//! ## Setup
//!
//! 1. Create an application at <https://discord.com/developers/applications>.
//! 2. Copy the application id and the public key into `autumn.toml`:
//!
//!    ```toml
//!    [discord]
//!    application_id = "123456789012345678"
//!    public_key = "<64 hex chars>"
//!    ```
//!
//! 3. Create a bot user, copy its token, and export it:
//!
//!    ```sh
//!    export DISCORD_BOT_TOKEN="..."
//!    ```
//!
//! 4. Run the bot, then set the interactions endpoint URL in the developer
//!    portal to `https://<your-host>/discord/interactions`.
//! 5. Register the `/ping` command once:
//!
//!    ```sh
//!    cargo run --example register_commands
//!    ```
//!
//! Then type `/ping` in any server with the bot.

use autumn_plugin_discord::{
    Command, CommandContext, DiscordPlugin, InteractionResponse, MessagePayload,
};
use autumn_web::prelude::*;

async fn ping(_ctx: CommandContext) -> AutumnResult<InteractionResponse> {
    Ok(InteractionResponse::channel_message(
        MessagePayload::content("Pong!"),
    ))
}

#[autumn_web::main]
async fn main() {
    autumn_web::app()
        .plugin(
            DiscordPlugin::new().command(Command::new("ping", "Replies with Pong!").handler(ping)),
        )
        .run()
        .await;
}
