//! Add the viewer's sign-off to a founding.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::FoundingId;

use crate::handlers::current_user::current_user;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// Add the viewer's sign-off to a founding. When this is the ninth, the demos is
/// founded and we redirect straight to it; otherwise back to the petition page.
pub async fn sign_founding(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in to sign off".to_string());
    };
    match state.founding.sign_founding(FoundingId(id), user.id).await {
        Ok(Some(demos)) => Redirect::to(&format!("/d/{}", demos.slug)).into_response(),
        Ok(None) => Redirect::to(&format!("/found/{id}")).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
