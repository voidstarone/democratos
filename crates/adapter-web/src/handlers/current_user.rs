//! Resolve the acting user from the signed session cookie.

use axum::http::HeaderMap;
use domain::{User, UserId};

use crate::handlers::cookie_value::cookie_value;
use crate::AppState;

pub(crate) async fn current_user(state: &AppState, headers: &HeaderMap) -> Option<User> {
    let raw = cookie_value(headers, "uid")?;
    // Only an id carrying a valid signature is honoured; a hand-written `uid=1`
    // fails verification and is treated as no session at all.
    let (id, expires_at) = state.session.verify(&raw)?;
    // An expired session is no session — the signed expiry is checked server-side
    // against the clock, so a captured cookie stops working once it lapses even if
    // the browser is coerced into resending it past its `Max-Age`.
    if state.services.clock.now().0 >= expires_at {
        return None;
    }
    state.services.users.get(UserId(id)).await.ok().flatten()
}
