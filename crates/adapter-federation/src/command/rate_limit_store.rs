//! The backend a rate limiter counts against — in-memory for dev, Postgres for a
//! durable cap shared across every replica.

use async_trait::async_trait;

/// Counts one attempt against a fixed-window `bucket` and reports whether it is
/// within `max_per_window`. Implementations must be atomic per bucket so the cap
/// holds under concurrency.
#[async_trait]
pub trait RateLimitStore: Send + Sync {
    /// Returns `true` to admit (and counts it), `false` if the bucket is over its cap
    /// for the current window.
    async fn admit(&self, bucket: &str, max_per_window: u32, window_secs: i64, now: i64) -> bool;
}

/// Durable, Postgres-backed limiter — the cap holds across restarts and across every
/// replica sharing the database. Fails **open** (admits) on a store error: a DB
/// outage already fails the underlying mint/authenticate use-case, so there is no
/// bypass to gain, and a transient blip must not lock every user out.
#[async_trait]
impl RateLimitStore for adapter_store_postgres::PostgresStore {
    async fn admit(&self, bucket: &str, max_per_window: u32, window_secs: i64, now: i64) -> bool {
        match self
            .admit_rate(bucket, max_per_window as i32, window_secs, now)
            .await
        {
            Ok(admit) => admit,
            Err(e) => {
                eprintln!("⚠ rate limiter store error (failing open) on {bucket}: {e}");
                true
            }
        }
    }
}
