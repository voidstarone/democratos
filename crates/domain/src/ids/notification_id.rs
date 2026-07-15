//! Identifies an in-app notification.

use serde::{Deserialize, Serialize};

/// Identifies an in-app notification.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct NotificationId(pub u64);

impl std::fmt::Display for NotificationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
