//! A process-local [`RateLimitStore`] for dev/tests — not shared across replicas.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::command::rate_limit_store::RateLimitStore;

/// A process-local fixed-window limiter. Correct for a single node / dev / tests;
/// production uses the durable Postgres-backed [`RateLimitStore`] so the cap holds
/// across replicas and restarts.
#[derive(Default)]
pub struct InMemoryRateLimitStore {
    windows: Mutex<HashMap<String, Window>>,
}

struct Window {
    start: i64,
    count: u32,
}

impl InMemoryRateLimitStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RateLimitStore for InMemoryRateLimitStore {
    async fn admit(&self, bucket: &str, max_per_window: u32, window_secs: i64, now: i64) -> bool {
        let window_secs = window_secs.max(1);
        let mut windows = self.windows.lock().unwrap();
        let w = windows.entry(bucket.to_string()).or_insert(Window {
            start: now,
            count: 0,
        });
        if now - w.start >= window_secs {
            w.start = now;
            w.count = 0;
        }
        if w.count >= max_per_window {
            return false;
        }
        w.count += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn caps_per_bucket_then_resets_after_the_window() {
        let store = InMemoryRateLimitStore::new();
        assert!(store.admit("mint:4", 2, 60, 1_000).await);
        assert!(store.admit("mint:4", 2, 60, 1_010).await);
        assert!(!store.admit("mint:4", 2, 60, 1_020).await, "over cap in the window");
        // A different bucket has its own budget.
        assert!(store.admit("mint:5", 2, 60, 1_020).await);
        // The window rolls over.
        assert!(store.admit("mint:4", 2, 60, 1_061).await, "new window resets");
    }
}
