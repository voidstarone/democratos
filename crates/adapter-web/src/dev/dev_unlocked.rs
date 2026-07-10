use axum::http::HeaderMap;

use crate::dev::dev_cookie::DEV_COOKIE;
use crate::dev::dev_cookie_value::DEV_COOKIE_VALUE;
use crate::handlers::cookie_value::cookie_value;
use crate::AppState;

/// Both gates: dev mode is on *and* this browser has been unlocked. Every fake-
/// auth handler funnels through here; a closed gate becomes a `404`.
pub fn dev_unlocked(state: &AppState, headers: &HeaderMap) -> bool {
    state.dev_mode && cookie_value(headers, DEV_COOKIE).as_deref() == Some(DEV_COOKIE_VALUE)
}
