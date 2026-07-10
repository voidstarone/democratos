use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};

use federation::ChangeEvent;

use crate::http::bearer_ok::bearer_ok;
use crate::http::ingest_state::IngestState;

#[derive(serde::Deserialize)]
struct IngestBody {
    /// The pushing node id. Retained on the wire for diagnostics; the push path
    /// no longer keys a cursor by it (see `Replicator::apply_pushed`).
    #[allow(dead_code)]
    peer_node: i64,
    events: Vec<ChangeEvent>,
}

async fn ingest_handler(
    State(state): State<IngestState>,
    headers: HeaderMap,
    Json(body): Json<IngestBody>,
) -> Result<Json<u64>, (StatusCode, String)> {
    if !bearer_ok(state.token.as_deref(), &headers) {
        return Err((StatusCode::UNAUTHORIZED, "unauthorized".into()));
    }
    apply_ingest(&state, body).await
}

async fn apply_ingest(
    state: &IngestState,
    body: IngestBody,
) -> Result<Json<u64>, (StatusCode, String)> {
    // A push is a durability pre-apply — apply the rows idempotently but do NOT
    // advance the ordered puller's cursor (see `Replicator::apply_pushed`).
    let out = state
        .replicator
        .apply_pushed(&body.events)
        .await
        // Surface the reason so the owner (and the load harness) can see why a
        // quorum push failed instead of a bare 500.
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(out.applied))
}

/// The synchronous-ingest router — a standby mounts this to receive an owner's
/// vote events and apply them before acking.
pub fn ingest_router(state: IngestState) -> Router {
    Router::new()
        .route("/federation/ingest", post(ingest_handler))
        .with_state(state)
}
