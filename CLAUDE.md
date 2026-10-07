# CLAUDE.md — agent guidance for autumn-plugin-discord

## Commands

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR=~/workspace/autumn-arena/target   # shared with sibling arena builds; never cargo clean
export TMPDIR=~/workspace/.tmp-cargo                      # /tmp is a 512MB tmpfs

cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
```

Run the gates one at a time; the box is shared with other builds.

## Architecture

| Module | Owns | Depends on |
|---|---|---|
| `config` | `DiscordConfig`: `[discord]` TOML section + `DISCORD_BOT_TOKEN` env + builder overrides | `error`, `toml` |
| `error` | `Error` / `ErrorKind` / HTTP status mapping | `autumn_web::AutumnError`, `thiserror` |
| `types` | Discord payloads (interactions, registrations, responses) | `serde` |
| `verify` | Ed25519 verification of webhook signatures | `ed25519-dalek`, `hex`, `error` |
| `commands` | `Command` builder, `CommandRegistry`, dispatch to handlers | `types`, `client`, `error` |
| `client` | `DiscordClient` REST (register, follow-ups, token check) | `reqwest`, `types`, `error` |
| `interactions` | `POST /discord/interactions` route + `VerifiedInteraction` extractor | `verify`, `commands`, `plugin::DiscordRuntime`, `autumn_web` |
| `gateway` | `GatewayClient`: identify, heartbeat, dispatch (feature `gateway`) | `tokio-tungstenite`, `futures-util`, `error` |
| `health` | `DiscordHealth`: `HealthIndicator` via `GET /applications/@me` | `client`, `autumn_web::actuator` |
| `plugin` | `DiscordPlugin` (`Plugin` impl), `DiscordRuntime`, startup assembly | everything above |

Data flow for one slash command: Discord `POST`s `/discord/interactions` →
`VerifiedInteraction` checks the Ed25519 signature (401 on failure) → the
route answers PINGs directly → `CommandRegistry::dispatch` runs the handler
→ the handler returns `InteractionResponse` as JSON.

## Rules

- Never put the bot token in `autumn.toml`, in code, or in logs. It travels
  only through `DISCORD_BOT_TOKEN`.
- `unsafe_code = "forbid"` crate-wide (also in tests — use fixed test
  vectors, never `std::env::set_var`).
- No `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!` in production code;
  tests may use `expect`.
- Docs and comments in ASD-STE100 style: short sentences, active voice,
  simple present tense.
- Unit tests live in `src/<module>/tests.rs`. Tests must not need Discord
  credentials: use the RFC 8032 vector in `verify/tests.rs`, the fake
  gateway in `gateway/tests.rs`, and local TCP stubs for REST.
- Autumn API questions: ground against the docs MCP
  (`~/workspace/skills/autumn-mcp/bin/mcp.py`) or the vendored
  `autumn-web-0.8.0` sources — never from memory.
- `DiscordPlugin::contract` declares the supported `autumn-web` range
  (`SUPPORTED_AUTUMN_WEB` in `src/plugin.rs`). Change it together with the
  `autumn-web` requirement in `Cargo.toml`.
- `Plugin::build` cannot do async work. It installs an `on_startup` hook;
  the hook builds the `DiscordRuntime` and puts it in `AppState` extensions
  via `extension_or_insert_with`. (`AppBuilder::with_extension` values are
  discarded at `run()` — do not use them for runtime state.)
- The gateway is never auto-started: users spawn `GatewayClient::run`
  themselves.
