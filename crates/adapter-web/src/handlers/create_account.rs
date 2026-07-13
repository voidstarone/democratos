//! Register a new account and sign it straight in.

use axum::{
    extract::{Form, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::handlers::csrf_valid::csrf_valid;
use crate::handlers::redirect_with_cookie::redirect_with_cookie;
use crate::handlers::register_form::RegisterForm;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::safe_next::safe_next;
use crate::handlers::uid_cookie::uid_cookie;
use crate::AppState;

/// Register a new account with credentials and sign it straight in.
pub async fn create_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<RegisterForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    // Invitation-only closes this path entirely: accounts are minted only via a
    // valid invite token at /invite/accept. Bounce the visitor to the waitlist.
    if state
        .invite_only
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Redirect::to("/request-invite").into_response();
    }
    if !csrf_valid(&headers, &form.csrf_token) {
        return render_error(lang, None, "session expired — please try again".to_string());
    }
    // Route the sign-up through the minting gateway: on a trusted-issuer node (or
    // single-box) it mints locally; on a non-issuer federated node it forwards to a
    // trusted issuer, which mints the account in its own namespace so it replicates
    // fleet-wide. The returned id is the new global account id.
    match state
        .minter
        .mint_account(&form.handle, &form.email, &form.password)
        .await
    {
        // Return to where the visitor came from (e.g. the /found/:id petition),
        // falling back to home when there's no valid same-site target.
        Ok(id) => redirect_with_cookie(
            safe_next(&form.next).as_deref().unwrap_or("/"),
            uid_cookie(
                &state.session,
                id.0,
                state.services.clock.now().0,
                state.secure_cookies,
            ),
        ),
        Err(e) => render_error(lang, None, e.to_string()),
    }
}
