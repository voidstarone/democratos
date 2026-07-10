use axum::{
    extract::{Form, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::dev::create_form::CreateForm;
use crate::dev::dev_unlocked::dev_unlocked;
use crate::dev::login_as_handle::login_as_handle;
use crate::AppState;

/// The fake sign-in: log in as a handle with no password. This is exactly the
/// old passwordless `/session` behaviour, now fenced behind both dev gates.
pub async fn dev_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<CreateForm>,
) -> Response {
    if !dev_unlocked(&state, &headers) {
        return StatusCode::NOT_FOUND.into_response();
    }
    login_as_handle(&state, &form.handle).await
}
