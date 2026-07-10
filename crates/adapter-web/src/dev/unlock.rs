use axum::{
    extract::{Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};

use app::constant_time_eq;

use crate::dev::dev_cookie::DEV_COOKIE;
use crate::dev::dev_cookie_value::DEV_COOKIE_VALUE;
use crate::dev::unlock_query::UnlockQuery;
use crate::AppState;

/// Hand out the unlock cookie. Gated on `--dev` alone — this is how a dev browser
/// *gets* unlocked in the first place; a real server has `dev_mode` off and so
/// returns `404`, never issuing the cookie. Redirects home so the dev bar — now
/// unlocked — appears on the next page.
pub async fn unlock(State(state): State<AppState>, Query(q): Query<UnlockQuery>) -> Response {
    if !state.dev_mode {
        return StatusCode::NOT_FOUND.into_response();
    }
    // When an unlock secret is configured, `?key=` must match it (constant-time)
    // before we hand out the cookie. A missing/wrong key is a `404`, identical to
    // dev-off — the endpoint neither confirms it exists nor that a secret is set.
    if let Some(secret) = state.dev_unlock_secret.as_deref() {
        if !constant_time_eq(q.key.as_bytes(), secret.as_bytes()) {
            return StatusCode::NOT_FOUND.into_response();
        }
    }
    let cookie = format!(
        "{DEV_COOKIE}={DEV_COOKIE_VALUE}; Path=/; HttpOnly; SameSite=Lax{}",
        crate::handlers::secure_attr::secure_attr(state.secure_cookies)
    );
    let mut resp = Redirect::to("/").into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, value);
    }
    resp
}
