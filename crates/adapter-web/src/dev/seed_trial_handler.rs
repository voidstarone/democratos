//! `POST /dev/trial/seed` — build a fresh case and open its trial.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};

use crate::dev::dev_unlocked::dev_unlocked;
use crate::dev::seed_trial::seed_trial;
use crate::AppState;

pub async fn seed_trial_handler(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !dev_unlocked(&state, &headers) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match seed_trial(&state).await {
        Ok(trial) => Redirect::to(&format!("/dev/trial?trial={}", trial.id.0)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}
