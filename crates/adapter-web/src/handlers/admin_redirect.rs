//! Redirect back to the review queue, preserving the admin secret and leaving a
//! short outcome code for the page to surface.

use axum::response::{IntoResponse, Redirect, Response};

/// Back to `/review-queue`, carrying the `key` (so the operator stays
/// authenticated) and a `msg` outcome code. The admin secret is expected to be
/// URL-safe (hex/base64url), so it is placed in the query verbatim — the same
/// convention the dev-unlock link uses.
pub(crate) fn admin_redirect(key: &str, msg: &str) -> Response {
    Redirect::to(&format!("/review-queue?key={key}&msg={msg}")).into_response()
}
