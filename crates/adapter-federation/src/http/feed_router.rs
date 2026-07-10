use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use federation::ChangeEvent;

use crate::http::bearer_ok::bearer_ok;
use crate::http::feed_state::FeedState;
use crate::changes_since;

#[derive(Deserialize)]
struct ChangesQuery {
    since: Option<i64>,
    limit: Option<i64>,
}

fn check_token(state: &FeedState, headers: &HeaderMap) -> Result<(), StatusCode> {
    bearer_ok(state.token.as_deref(), headers)
        .then_some(())
        .ok_or(StatusCode::UNAUTHORIZED)
}

async fn changes_handler(
    State(state): State<FeedState>,
    headers: HeaderMap,
    Query(q): Query<ChangesQuery>,
) -> Result<Json<Vec<ChangeEvent>>, StatusCode> {
    check_token(&state, &headers)?;
    let since = q.since.unwrap_or(0);
    let limit = q.limit.unwrap_or(500).clamp(1, 5_000);
    let events = changes_since(
        &state.store,
        &state.keypair,
        state.registry.as_ref(),
        since,
        limit,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(events))
}

/// The federation feed router — mount on a node-only address.
pub fn feed_router(state: FeedState) -> Router {
    Router::new()
        .route("/federation/changes", get(changes_handler))
        .with_state(state)
}
