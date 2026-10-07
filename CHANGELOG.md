# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Changed

- Upgrade to `autumn-web` 0.8 (was 0.7). Apps on `autumn-web` 0.7 must stay
  on 0.1.0 of this plugin.

### Added

- `DiscordPlugin` implements `Plugin::contract`: it declares support for
  `autumn-web` 0.8. A mismatched framework fails at registration with a
  diagnostic that names both versions.
- Tests run the `autumn_web::plugin_conformance` harness on the plugin's
  routes (`/discord` prefix, route attribution, contract).

## [0.1.0] - 2026-09-29

### Added

- `DiscordPlugin`: one-line install — declares the `[discord]` config
  section, mounts `POST /discord/interactions`, resolves config on startup,
  and registers a `"discord"` health indicator.
- Slash commands as data: `Command::new(name, description)` with options
  and async handlers (`CommandContext -> AutumnResult<InteractionResponse>`).
- `VerifiedInteraction` extractor: Ed25519 signature verification
  (`X-Signature-Ed25519` / `X-Signature-Timestamp`) that fails closed with
  401; usable on custom endpoints.
- Typed payloads: `Interaction`, `ApplicationCommandData`,
  `CommandOptionValue`, `CommandRegistration`, `InteractionResponse`,
  `MessagePayload` (including ephemeral messages).
- `DiscordClient` REST: `register_commands` (bulk overwrite),
  `create_followup` (deferred answers), `check_token`
  (`GET /applications/@me`).
- `GatewayClient`: a real minimal gateway client — connect, `Hello`,
  `Identify`, heartbeat on the negotiated interval, event dispatch to
  `EventHandler`. Reconnects with a fresh session on drops; no resume.
- `DiscordHealth`: Autumn `HealthIndicator` backed by the token check.
- Layered config: `autumn.toml` `[discord]` (`application_id`,
  `public_key`), `DISCORD_BOT_TOKEN` from the environment only, builder
  overrides via `.configure(|c| ...)`.
- Examples: `ping` (the `/ping` bot) and `register_commands`.
- Unit tests need no Discord credentials: RFC 8032 signature vectors,
  dispatch routing, payload parsing, a fake in-process gateway, and local
  TCP stubs for REST.

### Known gaps

- Gateway session resume is not implemented (fresh `Identify` on
  reconnect).
- No guild-scoped command registration helper yet.
