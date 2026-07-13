//! The sign-in page handler.

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

/// The sign-in page (email + password). A signed-in visitor is bounced home —
/// there's nothing to do here once you have a session.
pub async fn signin_page(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
    headers: HeaderMap,
) -> Response {
    let lang = resolve_lang(&headers);
    let next = query.next.as_deref().and_then(safe_next).unwrap_or_default();
    if current_user(&state, &headers).await.is_some() {
        return Redirect::to(if next.is_empty() { "/" } else { &next }).into_response();
    }
    let (csrf_token, set_cookie) = issue_csrf(&headers, state.secure_cookies);
    render_with_cookie(
        SignInView {
            t: lang.strings(),
            lang: lang.code(),
            current_user: None,
            mode: AuthMode::SignIn,
            csrf_token,
            next,
        },
        set_cookie,
    )
}
