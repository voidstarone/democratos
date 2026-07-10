//! The bootstrap phase of a demos.

use serde::{Deserialize, Serialize};

/// The bootstrap phase of a demos, derived purely from its voter count.
///
/// Small demos are where capture is easiest and percentage-math is weakest, so
/// new demos run on "training wheels" until self-governance is meaningful.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Phase {
    /// 1–9 voters. No constitutional amendments; platform-default criteria.
    Seed,
    /// 10–24 voters. Amendments allowed but under stricter thresholds.
    Chartering,
    /// 25+ voters. Full self-governance; percentage math now works naturally.
    Sovereign,
}

impl Phase {
    pub const CHARTERING_AT: u64 = 10;
    pub const SOVEREIGN_AT: u64 = 25;

    pub fn from_voter_count(voters: u64) -> Phase {
        if voters >= Self::SOVEREIGN_AT {
            Phase::Sovereign
        } else if voters >= Self::CHARTERING_AT {
            Phase::Chartering
        } else {
            Phase::Seed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_boundaries() {
        assert_eq!(Phase::from_voter_count(0), Phase::Seed);
        assert_eq!(Phase::from_voter_count(9), Phase::Seed);
        assert_eq!(Phase::from_voter_count(10), Phase::Chartering);
        assert_eq!(Phase::from_voter_count(24), Phase::Chartering);
        assert_eq!(Phase::from_voter_count(25), Phase::Sovereign);
        assert_eq!(Phase::from_voter_count(10_000), Phase::Sovereign);
    }
}
