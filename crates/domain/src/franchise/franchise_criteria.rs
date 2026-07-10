//! A demos's franchise constitution.

use serde::{Deserialize, Serialize};

/// A demos's franchise constitution: the bar a member must clear to become a
/// voter. Amendable by constitutional vote (Layer 3).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FranchiseCriteria {
    pub min_account_age_days: i64,
    pub min_membership_days: i64,
    pub min_contribution: i64,
}

impl FranchiseCriteria {
    /// The cautious platform default every new demos starts from.
    pub fn platform_default() -> Self {
        Self {
            min_account_age_days: 30,
            min_membership_days: 14,
            min_contribution: 5,
        }
    }
}
