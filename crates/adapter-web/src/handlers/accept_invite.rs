//! Redeem an invite: mint the account bound to the invited email, then sign in.

use axum::{
    extract::{Form, State},
    http::HeaderMap,
    response::Response,
};

use crate::handlers::accept_invite_form::AcceptInviteForm;
use crate::handlers::csrf_valid::csrf_valid;
use crate::handlers::issue_csrf::issue_csrf;
use crate::handlers::redirect_with_cookie::redirect_with_cookie;
use crate::handlers::render_error::render_error;
use crate::handlers::render_with_cookie::render_with_cookie;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::uid_cookie::uid_cookie;
use crate::views::accept_invite_view::AcceptInviteView;
use crate::AppState;

/// Finish an invited sign-up. Re-validates the token (never trusting the posted
/// email — it is read from the invite), mints the account through the same
/// gateway as open registration, consumes the invite, and signs the new account
/// straight in.
pub async fn accept_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<AcceptInviteForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    if !csrf_valid(&headers, &form.csrf_token) {
        return render_error(lang, None, "session expired — please try again".to_string());
    }

    // The token is the authority for *which* email this account gets — re-check it
    // and read the email from the stored request, not from anything the client sent.
    let request = match state.services.validate_invite_token(&form.token).await {
        Ok(request) => request,
        Err(_) => {
            return render_error(
                lang,
                None,
                "this invite link is invalid or has expired".to_string(),
            )
        }
    };

    match state
        .minter
        .mint_account(&form.handle, &request.email, &form.password)
        .await
    {
        Ok(id) => {
            // Consume the invite so the link can't be reused. If this write fails
            // the account still exists and the email is now taken, so a replay
            // would be rejected on uniqueness anyway — log and proceed.
            if let Err(e) = state.services.mark_invite_accepted(request.id).await {
                eprintln!("invite {} accepted but not marked consumed: {e}", request.id);
            }
            redirect_with_cookie(
                "/",
                uid_cookie(
                    &state.session,
                    id.0,
                    state.services.clock.now().0,
                    state.secure_cookies,
                ),
            )
        }
        Err(e) => {
            // A bad handle/password — let them fix it without losing the token.
            let (csrf_token, set_cookie) = issue_csrf(&headers, state.secure_cookies);
            render_with_cookie(
                AcceptInviteView {
                    t: lang.strings(),
                    lang: lang.code(),
                    current_user: None,
                    email: request.email,
                    token: form.token,
                    csrf_token,
                    error: Some(e.to_string()),
                    handle: form.handle,
                },
                set_cookie,
            )
        }
    }
}
