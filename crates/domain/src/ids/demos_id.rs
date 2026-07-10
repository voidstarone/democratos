//! Identifies a demos.

use serde::{Deserialize, Serialize};

/// Identifies a demos (community).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct DemosId(pub u64);

impl std::fmt::Display for DemosId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
