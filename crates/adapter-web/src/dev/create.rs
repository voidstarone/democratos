use axum::{
    extract::{Form, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};

use crate::dev::create_form::CreateForm;
use crate::dev::dev_unlocked::dev_unlocked;
use crate::dev::login_as_handle::login_as_handle;
use crate::AppState;

/// Create a fresh test account (or reuse one with the same handle) and switch
/// to it immediately.
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<CreateForm>,
) -> Response {
    if !dev_unlocked(&state, &headers) {
        return StatusCode::NOT_FOUND.into_response();
    }
    login_as_handle(&state, &form.handle).await
}
