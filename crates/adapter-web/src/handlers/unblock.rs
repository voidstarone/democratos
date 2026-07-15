//! Lift a personal block on another account.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::current_user::current_user;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// `POST /u/:handle/unblock` — the signed-in viewer lifts their block on the
/// account `handle`, and its content reappears in their feeds and threads.
pub async fn unblock(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(viewer) = current_user(&state, &headers).await else {
        return Redirect::to("/signin").into_response();
    };
    let target = match state.services.user_by_handle(&handle).await {
        Ok(Some(u)) => u,
        Ok(None) => return render_error(lang, Some(viewer.handle), "no such user".to_string()),
        Err(e) => return render_error(lang, Some(viewer.handle), e.to_string()),
    };
    if let Err(e) = state.services.unblock_user(viewer.id, target.id).await {
        return render_error(lang, Some(viewer.handle), e.to_string());
    }
    Redirect::to(&format!("/u/{handle}")).into_response()
}
