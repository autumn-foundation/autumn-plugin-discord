//! Register the bot's slash commands with Discord.
//!
//! Run once after adding or changing commands:
//!
//! ```sh
//! export DISCORD_BOT_TOKEN="..."
//! cargo run --example register_commands
//! ```
//!
//! This bulk-overwrites the global commands, so removed commands disappear
//! from Discord too. Global commands take up to an hour to propagate; use
//! guild commands during development.

use autumn_plugin_discord::{API_BASE, Command, DiscordClient, DiscordConfig};

fn commands() -> Vec<Command> {
    vec![Command::new("ping", "Replies with Pong!")]
}

#[tokio::main]
async fn main() {
    let config = DiscordConfig::load().expect("autumn.toml [discord] loads");
    config.validate().expect("discord config is valid");

    let token = config
        .bot_token()
        .expect("DISCORD_BOT_TOKEN is set")
        .to_owned();
    let client = DiscordClient::new(API_BASE, token)
        .expect("client builds")
        .with_application_id(config.application_id.clone());

    let registrations: Vec<_> = commands().iter().map(Command::registration).collect();
    let registered = client
        .register_commands(&registrations)
        .await
        .expect("registration succeeds");
    println!("registered {} command(s)", registered.len());
    for command in registered {
        let name = command.get("name").and_then(|n| n.as_str()).unwrap_or("?");
        println!("  - {name}");
    }
}
