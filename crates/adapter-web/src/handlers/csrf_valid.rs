//! Validate a submitted CSRF token against the browser's `csrf` cookie.

use axum::http::HeaderMap;

use app::constant_time_eq;

use crate::handlers::cookie_value::cookie_value;
use crate::handlers::csrf_cookie::CSRF_COOKIE;

/// Whether a submitted CSRF token matches the browser's `csrf` cookie. Missing
/// cookie or field, or any mismatch, fails closed. The comparison is
/// constant-time so a near-miss leaks nothing through timing.
pub(crate) fn csrf_valid(headers: &HeaderMap, submitted: &str) -> bool {
    match cookie_value(headers, CSRF_COOKIE) {
        Some(cookie) if !submitted.is_empty() => {
            constant_time_eq(cookie.as_bytes(), submitted.as_bytes())
        }
        _ => false,
    }
}
