//! Turn invitation-only access on or off from the admin review queue.

use std::net::SocketAddr;
use std::sync::atomic::Ordering;

use axum::{
    extract::{ConnectInfo, Form, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::handlers::admin_action_form::AdminActionForm;
use crate::handlers::admin_redirect::admin_redirect;
use crate::handlers::admin_unlocked::admin_unlocked;
use crate::handlers::csrf_valid::csrf_valid;
use crate::AppState;

/// Set invitation-only access to the submitted state, persisting it (so it
/// survives a restart) and updating the live in-memory flag the `/register` path
/// reads. Gated by subnet + secret (404 otherwise) and CSRF.
pub async fn toggle_invite_only(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<AdminActionForm>,
) -> Response {
    if !admin_unlocked(&state, peer.ip(), &form.key) {
        return StatusCode::NOT_FOUND.into_response();
    }
    if !csrf_valid(&headers, &form.csrf_token) {
        return admin_redirect(&form.key, "csrf");
    }
    // The toggle submits `enabled=on` to turn it on; its absence means off.
    let enable = form.enabled.is_some();
    match state.invites.set_invite_only(enable).await {
        Ok(()) => {
            // Persist succeeded — flip the hot-path flag to match.
            state.invite_only.store(enable, Ordering::Relaxed);
            admin_redirect(&form.key, if enable { "invite-on" } else { "invite-off" })
        }
        Err(_) => admin_redirect(&form.key, "error"),
    }
}
