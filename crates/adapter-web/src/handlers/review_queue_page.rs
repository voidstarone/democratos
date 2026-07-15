//! The operator's invite review queue (subnet + secret gated).

use std::net::SocketAddr;
use std::sync::atomic::Ordering;

use axum::{
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::handlers::admin_query::AdminQuery;
use crate::handlers::admin_unlocked::admin_unlocked;
use crate::handlers::issue_csrf::issue_csrf;
use crate::handlers::render_with_cookie::render_with_cookie;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::invite_queue_item::InviteQueueItem;
use crate::views::review_queue_view::ReviewQueueView;
use crate::AppState;

/// Render the pending-invite queue for an operator on an allowed subnet holding
/// the admin secret. Any gate failure returns a bare `404`, so the page is
/// invisible to everyone else.
pub async fn review_queue_page(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<AdminQuery>,
) -> Response {
    if !admin_unlocked(&state, peer.ip(), &query.key) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let lang = resolve_lang(&headers);

    let now = state.services.clock.now();
    let items = match state.invites.list_pending_invites().await {
        Ok(pending) => pending
            .into_iter()
            .map(|r| InviteQueueItem {
                id: r.id.0,
                email: r.email,
                note: r.note.unwrap_or_default(),
                waited_days: now.days_since(r.requested_at),
            })
            .collect(),
        // Don't leak a store error to a 404-gated page; show an empty queue and
        // log for the operator.
        Err(e) => {
            eprintln!("review queue: could not list pending invites: {e}");
            Vec::new()
        }
    };

    let (csrf_token, set_cookie) = issue_csrf(&headers, state.secure_cookies);
    render_with_cookie(
        ReviewQueueView {
            t: lang.strings(),
            lang: lang.code(),
            current_user: None,
            key: query.key,
            csrf_token,
            invite_only: state.invite_only.load(Ordering::Relaxed),
            msg: query.msg,
            items,
        },
        set_cookie,
    )
}
