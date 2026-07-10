//! The sign-in page handler.

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::current_user::current_user;
use crate::handlers::issue_csrf::issue_csrf;
use crate::handlers::render_with_cookie::render_with_cookie;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::auth_mode::AuthMode;
use crate::views::sign_in_view::SignInView;
use crate::AppState;

/// The sign-in page (email + password). A signed-in visitor is bounced home —
/// there's nothing to do here once you have a session.
pub async fn signin_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let lang = resolve_lang(&headers);
    if current_user(&state, &headers).await.is_some() {
        return Redirect::to("/").into_response();
    }
    let (csrf_token, set_cookie) = issue_csrf(&headers, state.secure_cookies);
    render_with_cookie(
        SignInView {
            t: lang.strings(),
            lang: lang.code(),
            current_user: None,
            mode: AuthMode::SignIn,
            csrf_token,
        },
        set_cookie,
    )
}
