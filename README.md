# autumn-plugin-discord

A Discord bot framework for [Autumn](https://autumn-web.app) apps: declare
slash commands as data, mount the signed interactions webhook, and answer
with typed responses. No heavyweight bot framework — just
`ed25519-dalek` for signature verification and `reqwest` for REST.

## Quickstart

```toml
# Cargo.toml
[dependencies]
autumn-plugin-discord = { git = "https://github.com/madmax983/autumn-plugin-discord" }
```

```toml
# autumn.toml
[discord]
application_id = "123456789012345678"
public_key = "<64 hex chars from the developer portal>"
```

```sh
export DISCORD_BOT_TOKEN="<bot token from the developer portal>"
```

```rust
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
            DiscordPlugin::new()
                .command(Command::new("ping", "Replies with Pong!").handler(ping)),
        )
        .run()
        .await;
}
```

Then, in the [Discord developer portal](https://discord.com/developers/applications):

1. Set the **Interactions Endpoint URL** to `https://<your-host>/discord/interactions`.
2. Register the command once: `cargo run --example register_commands`.
3. Type `/ping` in a server with the bot.

## What the plugin installs

| Piece | Detail |
|---|---|
| Config section | `[discord]` — `application_id`, `public_key`. The bot token comes only from `DISCORD_BOT_TOKEN`. |
| Route | `POST /discord/interactions` — verifies the Ed25519 signature, answers PINGs, dispatches slash commands. |
| Extractor | `VerifiedInteraction` — mount your own endpoint when `/discord/interactions` does not fit. |
| Health | A `"discord"` health indicator backed by `GET /applications/@me`. |
| REST client | `DiscordClient` — register commands, send follow-ups for deferred interactions. |
| Gateway | `GatewayClient` — minimal websocket client: identify, heartbeat, event dispatch. Spawn it yourself; the plugin does not auto-connect. |

## Deferred responses

Answer within 3 seconds, or acknowledge and follow up later:

```rust
use autumn_plugin_discord::{InteractionResponse, MessagePayload};

async fn slow(ctx: CommandContext) -> AutumnResult<InteractionResponse> {
    let token = ctx.interaction.token.clone();
    let client = ctx.client.clone();
    tokio::spawn(async move {
        // ... long work ...
        let _ = client
            .create_followup(&token, &MessagePayload::content("Done!"))
            .await;
    });
    Ok(InteractionResponse::deferred())
}
```

## Crate layout

| Module | Purpose |
|---|---|
| `config` | Layered `[discord]` config: file, env, builder overrides |
| `commands` | Slash-command declarations (`Command`), the `CommandRegistry`, dispatch |
| `interactions` | The webhook route and the `VerifiedInteraction` extractor |
| `verify` | Ed25519 signature verification (`X-Signature-Ed25519` / `X-Signature-Timestamp`) |
| `client` | Discord REST client |
| `gateway` | Minimal gateway client (feature `gateway`, on by default) |
| `health` | `GET /applications/@me` readiness check |
| `plugin` | `DiscordPlugin` (`::new()`, `.command()`, `.configure()`) and `DiscordRuntime` |
| `error` | `Error` + `ErrorKind` + mapping to HTTP status codes |

## Docs and comments

Written in ASD-STE100 style: short sentences, active voice, simple present
tense.

## Known issues

- The gateway client does not resume sessions. A dropped connection
  reconnects with a fresh `Identify`. Bots that need resume should extend
  `gateway.rs` or bring their own client.
- `register_commands` overwrites **global** commands. Global commands take up
  to an hour to propagate; guild-scoped registration is not wrapped yet.

## Build status

All three quality gates pass as of 2026-09-29:

```sh
cargo fmt --all -- --check        # clean
cargo clippy --locked --all-targets --all-features -- -D warnings  # clean
cargo test --locked --all-targets --all-features    # 50 passed, 0 failed
```

## License

Apache-2.0. See `LICENSE`.
