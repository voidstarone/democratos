//! Whether a request is the JS lazy-loader asking for a bare feed slice.

use axum::http::HeaderMap;

/// Whether this request is the JS lazy-loader asking for a bare feed slice rather
/// than a full page. Mirrors the vote handlers' `X-Requested-With` contract; a
/// plain browser navigation (no JS) never sets it and so gets the whole page.
pub(crate) fn wants_fragment(headers: &HeaderMap) -> bool {
    headers.contains_key("x-requested-with")
}
