use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

use crate::rate_limit::bucket::Bucket;

/// When the table grows past this many live keys, opportunistically evict the
/// expired ones so a churn of distinct IPs can't grow it without bound.
const PRUNE_THRESHOLD: usize = 10_000;

/// A fixed-window counter per `(ip, bucket)`.
#[derive(Default)]
pub struct RateLimiter {
    windows: Mutex<HashMap<(IpAddr, Bucket), Window>>,
}

/// One counter: how many hits so far in the window that opened at `started`.
struct Window {
    count: u32,
    started: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// Record a hit for `(ip, bucket)` at time `now` and report whether it is
    /// within the allowance. The window resets once `window` has elapsed since it
    /// opened, so a well-behaved client's count never accumulates across windows.
    /// `now` is a parameter (not read internally) so the fixed-window behaviour is
    /// unit-testable without sleeping.
    pub fn check_at(&self, ip: IpAddr, bucket: Bucket, now: Instant) -> bool {
        let (limit, window) = bucket.limits();
        let mut map = self.windows.lock().expect("rate-limit mutex poisoned");
        if map.len() > PRUNE_THRESHOLD {
            map.retain(|_, w| now.duration_since(w.started) < window);
        }
        let entry = map.entry((ip, bucket)).or_insert(Window {
            count: 0,
            started: now,
        });
        if now.duration_since(entry.started) >= window {
            entry.count = 0;
            entry.started = now;
        }
        entry.count = entry.count.saturating_add(1);
        entry.count <= limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limit::auth_max_requests::AUTH_MAX_REQUESTS;
    use crate::rate_limit::auth_window::AUTH_WINDOW;
    use std::net::Ipv4Addr;
    use std::time::Duration;

    #[test]
    fn allows_up_to_the_limit_then_blocks_within_a_window() {
        let rl = RateLimiter::new();
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let t0 = Instant::now();

        // The first AUTH_MAX_REQUESTS hits are allowed…
        for _ in 0..AUTH_MAX_REQUESTS {
            assert!(rl.check_at(ip, Bucket::Auth, t0));
        }
        // …and the next one, still inside the window, is refused.
        assert!(!rl.check_at(ip, Bucket::Auth, t0));
        assert!(!rl.check_at(ip, Bucket::Auth, t0 + Duration::from_secs(1)));
    }

    #[test]
    fn resets_after_the_window_elapses() {
        let rl = RateLimiter::new();
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let t0 = Instant::now();

        for _ in 0..AUTH_MAX_REQUESTS {
            assert!(rl.check_at(ip, Bucket::Auth, t0));
        }
        assert!(!rl.check_at(ip, Bucket::Auth, t0));

        // Once the window has fully elapsed the counter starts fresh.
        let later = t0 + AUTH_WINDOW + Duration::from_secs(1);
        assert!(rl.check_at(ip, Bucket::Auth, later));
    }

    #[test]
    fn buckets_and_ips_are_counted_independently() {
        let rl = RateLimiter::new();
        let a = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 1));
        let b = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 2));
        let t0 = Instant::now();

        // Exhaust A's auth allowance.
        for _ in 0..AUTH_MAX_REQUESTS {
            assert!(rl.check_at(a, Bucket::Auth, t0));
        }
        assert!(!rl.check_at(a, Bucket::Auth, t0));

        // A different IP is unaffected…
        assert!(rl.check_at(b, Bucket::Auth, t0));
        // …and A's *write* bucket is a separate, still-open allowance.
        assert!(rl.check_at(a, Bucket::Write, t0));
    }
}
