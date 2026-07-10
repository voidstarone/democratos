//! Identifies a pending founding petition.

use serde::{Deserialize, Serialize};

/// Identifies a pending founding petition (a demos not yet founded).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct FoundingId(pub u64);

impl std::fmt::Display for FoundingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
