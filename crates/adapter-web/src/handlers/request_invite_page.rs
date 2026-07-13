//! The public "ask for an invite" page.

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::current_user::current_user;
use crate::handlers::issue_csrf::issue_csrf;
use crate::handlers::render_with_cookie::render_with_cookie;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::request_invite_view::RequestInviteView;
use crate::AppState;

/// Render the waitlist request form. Anyone with the link can reach it; a signed-
/// in visitor already has an account, so send them home.
pub async fn request_invite_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let lang = resolve_lang(&headers);
    if current_user(&state, &headers).await.is_some() {
        return Redirect::to("/").into_response();
    }
    let (csrf_token, set_cookie) = issue_csrf(&headers, state.secure_cookies);
    render_with_cookie(
        RequestInviteView {
            t: lang.strings(),
            lang: lang.code(),
            current_user: None,
            csrf_token,
            submitted: false,
            error: None,
            email: String::new(),
            note: String::new(),
        },
        set_cookie,
    )
}
