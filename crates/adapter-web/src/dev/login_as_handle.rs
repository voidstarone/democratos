use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::dev::no_content_with_cookie::no_content_with_cookie;
use crate::handlers::uid_cookie::uid_cookie;
use crate::AppState;

/// Sign in as `handle` (creating the account if new) with no password. Shared by
/// the dev bar's "create" button and the passwordless `/dev/session` form.
///
/// The switcher only ever deals in franchise-barred "puppet" accounts: a *new*
/// handle is minted barred, and an *existing* handle is accepted only if it is
/// already barred. This means the fake sign-in can never be pointed at a real
/// member's account, and everything it can reach is a permanent non-voter.
pub async fn login_as_handle(state: &AppState, handle: &str) -> Response {
    let handle = handle.trim();
    if handle.is_empty() {
        return (StatusCode::BAD_REQUEST, "handle required").into_response();
    }
    let user = match state.services.users.by_handle(handle).await {
        Ok(Some(u)) if u.is_franchise_barred => u,
        // An existing, non-barred account is a real user — refuse to impersonate it.
        Ok(Some(_)) => {
            return (
                StatusCode::FORBIDDEN,
                "that handle is not a switchable puppet account",
            )
                .into_response()
        }
        Ok(None) => match state.services.register_barred_user(handle).await {
            Ok(u) => u,
            Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        },
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    no_content_with_cookie(uid_cookie(
        &state.session,
        user.id.0,
        state.services.clock.now().0,
        state.secure_cookies,
    ))
}
