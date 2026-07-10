//! Resolve a user's handle for display.

use domain::UserId;

use crate::AppState;

/// Resolve a user's handle for display, falling back to "user #id".
pub(crate) async fn handle_of(state: &AppState, id: UserId) -> String {
    state
        .services
        .users
        .get(id)
        .await
        .ok()
        .flatten()
        .map(|u| u.handle)
        .unwrap_or_else(|| format!("user #{}", id.0))
}
