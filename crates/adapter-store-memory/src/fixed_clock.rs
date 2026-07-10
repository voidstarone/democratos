//! A clock you can set and advance — for deterministic tests and demos seeding.

use std::sync::Mutex;

use app::Clock;
use domain::Timestamp;

/// A clock you can set and advance — for deterministic tests and demos seeding.
pub struct FixedClock {
    now: Mutex<Timestamp>,
}

impl FixedClock {
    pub fn at(now: Timestamp) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    pub fn set(&self, now: Timestamp) {
        *self.now.lock().expect("clock lock poisoned") = now;
    }

    pub fn advance_days(&self, days: i64) {
        let mut g = self.now.lock().expect("clock lock poisoned");
        *g = g.plus_days(days);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        *self.now.lock().expect("clock lock poisoned")
    }
}
