//! Lightweight JSON summary backing the reviewer nav badge.

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};

use crate::handlers::current_user::current_user;
use crate::AppState;

/// `GET /review/summary` → `{ "reviewer": bool, "open": n }`. Every page's nav
/// script calls this to decide whether to show the review badge; it identifies the
/// caller from the session cookie, so non-reviewers simply get `reviewer: false`.
/// Kept tiny and side-effect-free — it is polled on page load.
pub async fn review_summary(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let reviewer = current_user(&state, &headers)
        .await
        .map(|u| u.is_sensitive_reviewer)
        .unwrap_or(false);
    let open = if reviewer {
        state.services.open_case_count().await.unwrap_or(0)
    } else {
        0
    };
    Json(serde_json::json!({ "reviewer": reviewer, "open": open })).into_response()
}
