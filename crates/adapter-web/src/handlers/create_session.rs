//! Verify credentials and start a session.

use axum::{
    extract::{Form, State},
    http::HeaderMap,
    response::Response,
};

use crate::handlers::csrf_valid::csrf_valid;
use crate::handlers::login_form::LoginForm;
use crate::handlers::redirect_with_cookie::redirect_with_cookie;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::uid_cookie::uid_cookie;
use crate::AppState;

/// Verify an email + password and, on success, start a session. A failure re-
/// renders as the opaque "invalid email or password" so account existence never
/// leaks.
pub async fn create_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    // Reject a request whose CSRF token doesn't match the cookie *before* running
    // the (deliberately expensive) password verification — this both blocks login
    // CSRF and keeps a forged flood from doing Argon2 work.
    if !csrf_valid(&headers, &form.csrf_token) {
        return render_error(lang, None, "session expired — please try again".to_string());
    }
    match state
        .services
        .authenticate(&form.email, &form.password)
        .await
    {
        Ok(user) => redirect_with_cookie(
            "/",
            uid_cookie(
                &state.session,
                user.id.0,
                state.services.clock.now().0,
                state.secure_cookies,
            ),
        ),
        Err(e) => render_error(lang, None, e.to_string()),
    }
}
