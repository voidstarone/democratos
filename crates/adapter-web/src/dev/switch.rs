use axum::{
    extract::{Form, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use domain::UserId;

use crate::dev::dev_unlocked::dev_unlocked;
use crate::dev::no_content_with_cookie::no_content_with_cookie;
use crate::dev::switch_form::SwitchForm;
use crate::handlers::uid_cookie::uid_cookie;
use crate::AppState;

/// Point the session cookie at an existing account.
pub async fn switch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<SwitchForm>,
) -> Response {
    if !dev_unlocked(&state, &headers) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match state.services.users.get(UserId(form.id)).await {
        // Only a franchise-barred puppet is switchable; a real account is off-limits
        // even by id (and reported as "no such user" so ids aren't probed for reality).
        Ok(Some(user)) if user.is_franchise_barred => no_content_with_cookie(uid_cookie(
            &state.session,
            user.id.0,
            state.services.clock.now().0,
            state.secure_cookies,
        )),
        Ok(Some(_)) | Ok(None) => (StatusCode::NOT_FOUND, "no such user").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
