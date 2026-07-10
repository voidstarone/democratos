//! Register a new account and sign it straight in.

use axum::{
    extract::{Form, State},
    http::HeaderMap,
    response::Response,
};

use crate::handlers::csrf_valid::csrf_valid;
use crate::handlers::redirect_with_cookie::redirect_with_cookie;
use crate::handlers::register_form::RegisterForm;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::uid_cookie::uid_cookie;
use crate::AppState;

/// Register a new account with credentials and sign it straight in.
pub async fn create_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<RegisterForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    if !csrf_valid(&headers, &form.csrf_token) {
        return render_error(lang, None, "session expired — please try again".to_string());
    }
    match state
        .services
        .register_account(&form.handle, &form.email, &form.password)
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
