//! Source of "now".

use domain::Timestamp;

/// Source of "now". Injecting it keeps the time-based rules testable.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}
