//! The registration page handler.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::auth_query::AuthQuery;
use crate::handlers::current_user::current_user;
use crate::handlers::issue_csrf::issue_csrf;
use crate::handlers::render_with_cookie::render_with_cookie;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::safe_next::safe_next;
use crate::views::auth_mode::AuthMode;
use crate::views::sign_in_view::SignInView;
use crate::AppState;

/// The registration page (handle + email + password).
pub async fn register_page(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
    headers: HeaderMap,
) -> Response {
    let lang = resolve_lang(&headers);
    let next = query.next.as_deref().and_then(safe_next).unwrap_or_default();
    if current_user(&state, &headers).await.is_some() {
        return Redirect::to(if next.is_empty() { "/" } else { &next }).into_response();
    }
    // Invitation-only: open registration is closed — send visitors to the
    // waitlist instead. An accepted invite creates the account via /invite/accept.
    if state
        .invite_only
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Redirect::to("/request-invite").into_response();
    }
    let (csrf_token, set_cookie) = issue_csrf(&headers, state.secure_cookies);
    render_with_cookie(
        SignInView {
            t: lang.strings(),
            lang: lang.code(),
            current_user: None,
            mode: AuthMode::Register,
            csrf_token,
            next,
        },
        set_cookie,
    )
}
