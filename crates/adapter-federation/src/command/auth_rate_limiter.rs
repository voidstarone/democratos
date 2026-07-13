//! Rate limits on delegated login verification: per target handle AND per node.

use std::sync::Arc;

use crate::command::rate_limit_store::RateLimitStore;

/// Two caps on the delegated-auth path, both counted in a [`RateLimitStore`]
/// (durable/shared in production):
///
/// * **per target handle** — the brute-force guard: bounds guesses against a single
///   account. Keyed by the *exact* handle the issuer looks up
///   ([`Services::authenticate_by_handle`](app::Services::authenticate_by_handle)
///   trims but does NOT lowercase), so the bucket lines up 1:1 with the account under
///   attack — no cross-account interference, and no way to multiply guesses via
///   case/whitespace variants (they resolve to different accounts anyway).
/// * **per requesting node** — the spraying guard: bounds *total* login attempts a
///   node may relay across ALL handles, so an authenticated node can't spray common
///   passwords over thousands of accounts under the per-handle radar.
///
/// Both must admit for an attempt to proceed.
pub struct AuthRateLimiter {
    store: Arc<dyn RateLimitStore>,
    max_per_handle: u32,
    max_per_node: u32,
    window_secs: i64,
}

impl AuthRateLimiter {
    pub fn new(
        store: Arc<dyn RateLimitStore>,
        max_per_handle: u32,
        max_per_node: u32,
        window_secs: i64,
    ) -> Self {
        Self {
            store,
            max_per_handle,
            max_per_node,
            window_secs,
        }
    }

    /// Count a login attempt against `handle`, relayed by `node`, at `now` (unix
    /// seconds). Returns `true` only if BOTH the per-handle and per-node caps admit
    /// it. The handle is trimmed to match the issuer's account lookup exactly.
    pub async fn admit(&self, handle: &str, node: u16, now: i64) -> bool {
        // Per-handle first (tight). Only if it admits do we spend the node's broader
        // budget — so hammering one account fills only that account's bucket, not the
        // node's, and can't throttle the node's other legitimate users.
        self.store
            .admit(
                &format!("auth:{}", handle.trim()),
                self.max_per_handle,
                self.window_secs,
                now,
            )
            .await
            && self
                .store
                .admit(
                    &format!("authnode:{node}"),
                    self.max_per_node,
                    self.window_secs,
                    now,
                )
                .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryRateLimitStore;

    fn limiter(max_handle: u32, max_node: u32) -> AuthRateLimiter {
        AuthRateLimiter::new(Arc::new(InMemoryRateLimitStore::new()), max_handle, max_node, 300)
    }

    #[tokio::test]
    async fn per_handle_cap_is_case_sensitive_matching_the_account_lookup() {
        // The bucket keys on the EXACT handle the issuer looks up (trimmed, not
        // lowercased). So hammering "Alice" — a different account — does NOT consume
        // "alice"'s budget: no cross-case login DoS, and no way to multiply guesses.
        let rl = limiter(2, 100);
        assert!(rl.admit("alice", 1, 1_000).await);
        assert!(rl.admit("Alice", 1, 1_000).await, "different account, own budget");
        assert!(rl.admit("alice", 1, 1_000).await, "alice still has its 2nd guess");
        assert!(!rl.admit("alice", 1, 1_000).await, "alice over its own cap");
        assert!(rl.admit("Alice", 1, 1_000).await, "Alice unaffected by alice's cap");
        // Whitespace variants DO share the bucket (they resolve to the same account).
        assert!(!rl.admit("  alice ", 1, 1_000).await, "trimmed variant shares alice's bucket");
    }

    #[tokio::test]
    async fn per_node_cap_stops_password_spraying_across_many_handles() {
        // Spraying: one guess each against many DIFFERENT handles slips under every
        // per-handle cap. The per-node cap catches the total.
        let rl = limiter(10, 3);
        assert!(rl.admit("a", 7, 1_000).await);
        assert!(rl.admit("b", 7, 1_000).await);
        assert!(rl.admit("c", 7, 1_000).await);
        assert!(!rl.admit("d", 7, 1_000).await, "node 7 exhausted its total budget");
        // A different node still has its own budget.
        assert!(rl.admit("d", 8, 1_000).await);
    }
}
