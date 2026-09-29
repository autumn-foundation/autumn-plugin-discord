//! The Discord interactions webhook.
//!
//! `POST /discord/interactions` receives every interaction Discord sends:
//! PINGs, slash-command invocations, component interactions. The
//! [`VerifiedInteraction`] extractor verifies the Ed25519 signature and
//! parses the body; the route handler answers PINGs and dispatches commands
//! to their registered handlers.
//!
//! Mount your own endpoint with the extractor when the default path does not
//! fit — the plugin's own route is just this extractor plus dispatch.

use std::sync::Arc;

use autumn_web::reexports::axum::body::to_bytes;
use autumn_web::reexports::axum::extract::{FromRequest, Request};
use autumn_web::reexports::axum::http::request::Parts;
use autumn_web::{AppState, AutumnError, AutumnResult, Json, State, post, routes};

use crate::error::Error;
use crate::plugin::DiscordRuntime;
use crate::types::{Interaction, InteractionResponse, InteractionType};
use crate::verify::{SIGNATURE_HEADER, TIMESTAMP_HEADER, verify_interaction};

/// Maximum interaction body size the extractor reads (1 MiB).
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// An interaction whose Ed25519 signature already verified.
///
/// Use it in your own handlers to mount a custom interactions endpoint:
/// signature checks fail closed with 401 before your handler runs.
///
/// The runtime travels with the interaction so handlers can dispatch and
/// reach the REST client without another state lookup.
pub struct VerifiedInteraction {
    /// The parsed interaction.
    pub interaction: Interaction,
    /// The plugin runtime (public key, REST client, command registry).
    pub runtime: Arc<DiscordRuntime>,
}

/// Debug shows the interaction kind only; tokens stay out of logs.
impl std::fmt::Debug for VerifiedInteraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedInteraction")
            .field("kind", &self.interaction.kind)
            .finish_non_exhaustive()
    }
}

impl FromRequest<AppState> for VerifiedInteraction {
    type Rejection = AutumnError;

    async fn from_request(req: Request, state: &AppState) -> Result<Self, Self::Rejection> {
        let runtime = state
            .extension::<DiscordRuntime>()
            .ok_or_else(|| Error::MissingRuntime.to_autumn_error())?;
        let (parts, body) = req.into_parts();
        let signature = header_value(&parts, SIGNATURE_HEADER);
        let timestamp = header_value(&parts, TIMESTAMP_HEADER);
        let bytes = to_bytes(body, MAX_BODY_BYTES)
            .await
            .map_err(|_| AutumnError::bad_request_msg("interaction body too large"))?;
        verify_interaction(&runtime.public_key, signature, timestamp, &bytes)
            .map_err(|err| err.to_autumn_error())?;
        let interaction: Interaction =
            serde_json::from_slice(&bytes).map_err(|err| Error::Json(err).to_autumn_error())?;
        Ok(Self {
            interaction,
            runtime,
        })
    }
}

/// Read a header as `&str`, or `None` when missing or non-ASCII.
fn header_value<'a>(parts: &'a Parts, name: &str) -> Option<&'a str> {
    parts.headers.get(name)?.to_str().ok()
}

/// The plugin's interactions webhook.
#[post("/interactions")]
async fn interactions_webhook(
    State(_state): State<AppState>,
    VerifiedInteraction {
        interaction,
        runtime,
    }: VerifiedInteraction,
) -> AutumnResult<Json<InteractionResponse>> {
    // Discord PINGs the endpoint when the operator saves it in the portal.
    if interaction.kind == InteractionType::Ping {
        return Ok(Json(InteractionResponse::pong()));
    }
    let ctx = crate::commands::CommandContext {
        interaction,
        client: runtime.client.clone(),
    };
    let response = runtime.registry.dispatch(ctx).await?;
    Ok(Json(response))
}

/// The webhook route, for the plugin to mount under `/discord`.
pub(crate) fn webhook_routes() -> Vec<autumn_web::Route> {
    routes![interactions_webhook]
}

#[cfg(test)]
mod tests;
