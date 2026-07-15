//! Lightweight JSON summary backing the toolbar notification badge.

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};

use crate::handlers::current_user::current_user;
use crate::AppState;

/// `GET /notifications/summary` → `{ "signed_in": bool, "unread": n }`. The nav
/// script polls this on page load to decide whether to show the bell and its
/// unread pill; a signed-out visitor gets `signed_in: false` and no bell. Kept
/// tiny and side-effect-free.
pub async fn notifications_summary(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let unread = match current_user(&state, &headers).await {
        Some(user) => state
            .services
            .unread_notification_count(user.id)
            .await
            .unwrap_or(0),
        None => {
            return Json(serde_json::json!({ "signed_in": false, "unread": 0 })).into_response()
        }
    };
    Json(serde_json::json!({ "signed_in": true, "unread": unread })).into_response()
}
