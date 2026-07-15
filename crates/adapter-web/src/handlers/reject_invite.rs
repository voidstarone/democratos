//! Reject a pending request from the admin review queue.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Form, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::InviteId;

use app::ApproveInviteError;

use crate::handlers::admin_action_form::AdminActionForm;
use crate::handlers::admin_redirect::admin_redirect;
use crate::handlers::admin_unlocked::admin_unlocked;
use crate::handlers::csrf_valid::csrf_valid;
use crate::AppState;

/// Reject the request `id`. No email is sent. Gated by subnet + secret (404
/// otherwise) and CSRF.
pub async fn reject_invite(
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
    let Some(id) = form.id else {
        return admin_redirect(&form.key, "error");
    };
    let code = match state.invites.reject_invite(InviteId(id)).await {
        Ok(()) => "rejected",
        Err(ApproveInviteError::NotPending) => "not-pending",
        Err(_) => "error",
    };
    admin_redirect(&form.key, code)
}
