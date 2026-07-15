//! Block another account (a personal, one-directional mute).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::current_user::current_user;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// `POST /u/:handle/block` — the signed-in viewer blocks the account `handle`.
/// Blocking is unbounded and one-directional: it hides that account's content
/// from the viewer only. Redirects back to the profile so the button flips to
/// "Unblock". A SameSite=Lax session cookie is the CSRF defence, as for the other
/// mutating profile/community forms.
pub async fn block(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(handle): Path<String>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(viewer) = current_user(&state, &headers).await else {
        return Redirect::to("/signin").into_response();
    };
    let target = match state.accounts.user_by_handle(&handle).await {
        Ok(Some(u)) => u,
        Ok(None) => return render_error(lang, Some(viewer.handle), "no such user".to_string()),
        Err(e) => return render_error(lang, Some(viewer.handle), e.to_string()),
    };
    if let Err(e) = state.blocking.block_user(viewer.id, target.id).await {
        return render_error(lang, Some(viewer.handle), e.to_string());
    }
    Redirect::to(&format!("/u/{handle}")).into_response()
}
