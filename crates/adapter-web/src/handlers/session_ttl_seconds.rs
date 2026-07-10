//! Lifetime of a freshly-minted session cookie.

/// How long a freshly-minted session cookie stays valid (seconds). After this the
/// signed expiry is in the past, so [`current_user`](crate::handlers::current_user)
/// rejects it and the browser (via `Max-Age`) has already dropped it — a stolen
/// cookie has a bounded life.
pub(crate) const SESSION_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;
