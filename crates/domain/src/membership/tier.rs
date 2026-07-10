//! Citizenship tier within a demos.

use serde::{Deserialize, Serialize};

/// Citizenship tier within a demos.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Tier {
    /// Reading only; not joined.
    Lurker,
    /// Joined; accruing dwell time and contribution toward the franchise.
    Member,
    /// Enfranchised citizen: may vote.
    Voter,
}
