//! Identifies a sensitive-content review case.

use serde::{Deserialize, Serialize};

/// Identifies a sensitive-content review case — a platform-wide (extra-demos) case
/// gathering reviewer classifications of flagged content.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct SensitiveCaseId(pub u64);

impl std::fmt::Display for SensitiveCaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
