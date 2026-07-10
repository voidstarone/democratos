//! Join a community.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::render_error::render_error;
use crate::handlers::require_user_and_demos::require_user_and_demos;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

pub async fn join(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Response {
    let lang = resolve_lang(&headers);
    let (user, demos) = require_user_and_demos!(state, headers, lang, slug);
    if let Err(e) = state.services.join(user.id, demos.id).await {
        return render_error(lang, Some(user.handle), e.to_string());
    }
    Redirect::to(&format!("/d/{slug}")).into_response()
}
