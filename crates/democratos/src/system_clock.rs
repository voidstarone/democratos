//! A real wall-clock implementing the `Clock` port.

use std::time::{SystemTime, UNIX_EPOCH};

use app::Clock;
use domain::Timestamp;

/// A real wall-clock, implementing the `Clock` port. Lives in the composition
/// root because it is an environmental concern, not business logic.
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Timestamp(secs)
    }
}
