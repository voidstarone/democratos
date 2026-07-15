use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use crate::dev::dev_unlocked::dev_unlocked;
use crate::handlers::cookie_value::cookie_value;
use crate::AppState;

/// JSON snapshot the dev bar renders: who we're acting as, and every account
/// available to switch to. Returns `404` (the bar then stays hidden) unless both
/// dev gates are open.
pub async fn accounts(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !dev_unlocked(&state, &headers) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let current = cookie_value(&headers, "uid")
        .and_then(|v| state.session.verify(&v))
        .map(|(id, _expires_at)| id);
    // Only the franchise-barred puppet accounts are switchable, so the bar lists
    // exactly those — never a real member — matching what `switch` will accept.
    let users = state.accounts.list_users().await.unwrap_or_default();
    let users: Vec<_> = users
        .iter()
        .filter(|u| u.is_franchise_barred)
        .map(|u| serde_json::json!({ "id": u.id.0, "handle": u.handle }))
        .collect();
    Json(serde_json::json!({ "current": current, "users": users })).into_response()
}
