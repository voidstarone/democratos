//! A per-requesting-node rate limit on delegated account minting.

use std::sync::Arc;

use crate::command::rate_limit_store::RateLimitStore;

/// Bounds how many accounts a single requesting node may have a trusted issuer mint
/// within a window, so an authenticated-but-abusive node can't flood the federation
/// with accounts. Keyed by the *forwarding* node id (already authenticated by its
/// signature). Backed by a [`RateLimitStore`] — durable/shared in production so the
/// cap holds across replicas and restarts.
pub struct MintRateLimiter {
    store: Arc<dyn RateLimitStore>,
    max_per_window: u32,
    window_secs: i64,
}

impl MintRateLimiter {
    /// Allow `max_per_window` mints per `window_secs` from each requesting node.
    pub fn new(store: Arc<dyn RateLimitStore>, max_per_window: u32, window_secs: i64) -> Self {
        Self {
            store,
            max_per_window,
            window_secs,
        }
    }

    /// Record a mint attempt from `node` at `now` (unix seconds); `true` if within cap.
    pub async fn admit(&self, node: u16, now: i64) -> bool {
        self.store
            .admit(&format!("mint:{node}"), self.max_per_window, self.window_secs, now)
            .await
    }
}
