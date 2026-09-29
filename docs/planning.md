# Planning — autumn-plugin-discord

## Goal

A Discord bot framework for Autumn, in the fun lane: slash commands declared
as data, a signed interactions webhook, a REST client, and a minimal
gateway client. Shipped as a private `madmax983` arena crate; a candidate for
promotion to `autumn-foundation`.

## Non-goals (v0.1)

- A full high-level bot framework (serenity-style builders, caches).
- Gateway session resume.
- Guild-scoped command registration helpers.
- Component interactions / modal submit dispatch beyond parsing (the
  extractor parses them; routing only covers slash commands).

## Decisions

- `ed25519-dalek` 2.2 + `reqwest` 0.12, per the brief. No serenity/twilight.
- Signature verification fails closed: missing or malformed headers are 401.
- The bot token comes only from `DISCORD_BOT_TOKEN`. The config parser
  ignores a `bot_token` key in `[discord]`; the builder offers no setter.
- `Plugin::build` stays synchronous. Config resolution, validation, and
  runtime assembly happen in an `on_startup` hook; failure aborts boot with
  a clear message.
- Runtime sharing goes through `AppState::extension_or_insert_with`
  (`with_extension` values are discarded at `run()` — verified in
  autumn-web 0.7.0 sources).
- The gateway client is real (identify, heartbeat, dispatch) but minimal:
  reconnects with a fresh session, no resume. Documented as a gap.
- The plugin does not auto-start the gateway — the user spawns
  `GatewayClient::run` from their own startup hook.

## Autumn API grounding (2026-09-29)

Verified against the docs MCP and the vendored `autumn-web-0.7.0` sources:

- Tier-3 plugin shape: `impl Plugin for X { fn build(self, app: AppBuilder)
  -> AppBuilder }`, installed via `.plugin(...)`.
- `AppBuilder::config_section("discord")` exempts `[discord]` from strict
  config validation.
- Custom extractor pattern: `impl FromRequest<AppState>` (or
  `FromRequestParts<AppState>`) with `Rejection = AutumnError`.
- Route macros `#[post("/path")]` + `routes![handler]` produce
  `Vec<Route>`; `Route { path, handler: MethodRouter<AppState>, .. }` mounts
  into `axum::Router<AppState>` for `.nest("/discord", router)`.
- `AppBuilder::health_indicator(name, Arc<dyn HealthIndicator>)` for
  build-time registration; `AppState::health_indicator_registry().register`
  for runtime registration (used here, since the REST client only exists
  after startup).
- `#[apidoc]` + swagger-ui are built into Autumn; not reinvented here.

## Test strategy

No test needs Discord credentials or network:

- `verify`: RFC 8032 §7.1 test vector + tamper/wrong-key/missing-header
  cases.
- `types`: parse real-shaped interaction JSON; reject unknown types.
- `commands`: dispatch routing to the right handler; proptest over valid
  command names.
- `client` / `health`: local TCP stubs stand in for Discord's REST API.
- `gateway`: a fake in-process gateway (hello → identify → heartbeat →
  dispatch → clean close).
- `interactions`: the extractor against a test `AppState` with a signed
  PING, a tampered body, and a missing runtime.
- `config` / `plugin`: validation and builder precedence without env
  mutation (`unsafe_code = "forbid"` covers tests too).

## Future work

- Gateway resume (`session_id` + `seq` replay).
- Guild-scoped command registration.
- Component/modal dispatch routing.
- Rate-limit (429) retry with the `Retry-After` header.
