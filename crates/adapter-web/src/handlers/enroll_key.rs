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
    // Key enrolment is authoritative only on the account's HOME node — the one that
    // holds its credentials. A signing key is a *permanent*, first-key-only grant, so
    // letting a relaying node enrol one would let a malicious host substitute its own
    // key and forge this account's governance forever (surviving a password change).
    // Credentials never replicate, so `password_hash == None` here means this is not
    // the home node: refuse rather than write a local value replication would discard.
    // (A federated account must enrol on its home server.)
    if user.password_hash.is_none() {
        return render_error(
            lang,
            Some(user.handle),
            "signing keys can only be enrolled on your account's home server".to_string(),
        );
    }
    match state
        .services
        .enroll_public_key(user.id, form.public_key.trim())
        .await
    {
        Ok(()) => Redirect::to("/").into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
