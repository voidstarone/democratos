//! The "found a community" page handler.

use axum::{extract::State, http::HeaderMap, response::Response};

use crate::handlers::current_user::current_user;
use crate::handlers::render::render;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::found_view::FoundView;
use crate::AppState;

/// The "found a community" page. A dedicated page (not an inline form on the
/// home feed) so founding gets room for its own explanation.
pub async fn found_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    render(FoundView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: user.map(|u| u.handle),
    })
}
