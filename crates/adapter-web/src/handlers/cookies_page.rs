//! The cookie / privacy notice page handler.

use axum::{extract::State, http::HeaderMap, response::Response};

use crate::handlers::current_user::current_user;
use crate::handlers::render::render;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::cookies_view::CookiesView;
use crate::AppState;

/// The cookie notice. Reachable from the footer of every page, and readable
/// signed out — a visitor must be able to see what is stored before deciding to
/// register.
pub async fn cookies_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    render(CookiesView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: user.map(|u| u.handle),
    })
}
