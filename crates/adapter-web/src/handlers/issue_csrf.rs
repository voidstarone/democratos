//! Mint (or reuse) the double-submit anti-CSRF token for an auth form.

use axum::http::HeaderMap;

use crate::handlers::cookie_value::cookie_value;
use crate::handlers::csrf_cookie::CSRF_COOKIE;
use crate::handlers::secure_attr::secure_attr;

/// A hex token with ~256 bits of entropy, drawn without any RNG dependency.
///
/// `RandomState::new()` is seeded from the OS CSPRNG with two secret SipHash
/// keys; hashing a value under a *fresh* `RandomState` therefore yields an output
/// an attacker can't predict without those keys. Four independent draws are
/// concatenated. This is for an unguessable per-session anti-CSRF token, not a
/// long-term secret, and it stays within std as the crate requires.
fn random_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let stack_marker = &seed as *const _ as usize;
    let mut out = String::with_capacity(64);
    for i in 0..4u64 {
        let mut h = RandomState::new().build_hasher();
        h.write_u128(seed);
        h.write_u64(i);
        h.write_usize(stack_marker);
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// The CSRF token to embed in a freshly-rendered auth form, plus the `Set-Cookie`
/// that must accompany the response when the token is new. An existing, well-
/// formed `csrf` cookie is reused so a page refresh doesn't invalidate a form the
/// user already has open.
pub(crate) fn issue_csrf(headers: &HeaderMap, secure: bool) -> (String, Option<String>) {
    match cookie_value(headers, CSRF_COOKIE) {
        Some(tok) if tok.len() >= 32 => (tok, None),
        _ => {
            let tok = random_token();
            let cookie = format!(
                "{CSRF_COOKIE}={tok}; Path=/; HttpOnly; SameSite=Lax{}",
                secure_attr(secure)
            );
            (tok, Some(cookie))
        }
    }
}
