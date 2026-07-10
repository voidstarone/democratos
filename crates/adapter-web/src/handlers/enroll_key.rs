//! Enrol the signed-in account's signing key.

use axum::{
    extract::{Form, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::current_user::current_user;
use crate::handlers::enroll_key_form::EnrollKeyForm;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// Enrol the signed-in account's signing key. After this, the account's governance
/// actions must be signed by the matching secret key — so no node can forge them.
/// First-key-only: a key can't be silently replaced (see `Services::enroll_public_key`).
pub async fn enroll_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<EnrollKeyForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    match state
        .services
        .enroll_public_key(user.id, form.public_key.trim())
        .await
    {
        Ok(()) => Redirect::to("/").into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
