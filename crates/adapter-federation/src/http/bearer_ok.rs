use axum::http::HeaderMap;

/// Whether a request carries the expected `Bearer` token. `None` expected means
/// the check is disabled (local/dev). The comparison is constant-time so the
/// shared secret can't be recovered a byte at a time via response timing.
pub(crate) fn bearer_ok(expected: Option<&str>, headers: &HeaderMap) -> bool {
    let Some(expected) = expected else {
        return true; // no token configured — open (local/dev)
    };
    let presented = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    app::constant_time_eq(
        presented.as_bytes(),
        format!("Bearer {expected}").as_bytes(),
    )
}
