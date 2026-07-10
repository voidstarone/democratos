//! A user's membership record within a demos.

use serde::{Deserialize, Serialize};

use crate::{DemosId, Tier, Timestamp, UserId};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Membership {
    pub user_id: UserId,
    pub demos_id: DemosId,
    pub joined_at: Timestamp,
    pub tier: Tier,
    /// Under an active sanction (disqualifies from the franchise).
    pub sanctioned: bool,
    /// Contributions that existing voters reacted positively to. The mechanism
    /// that produces this score (and how it resists gaming) is an open question;
    /// the domain only consumes the resulting count.
    pub contribution: i64,
    /// When this member was admitted to the franchise, if ever. Used by the
    /// enfranchisement rate cap (Layer 2) to measure recent admissions.
    pub enfranchised_at: Option<Timestamp>,
    /// Per-member voting weight granted by the community (see
    /// [`crate::ProposalKind::GrantVoteWeight`]). Only consulted under the
    /// [`crate::VoteWeighting::ByRole`] scheme; `1` means an ordinary citizen.
    /// `#[serde(default = "..")]` defaults older datasets to an unweighted `1`.
    #[serde(default = "default_granted_weight")]
    pub granted_weight: u32,
}

fn default_granted_weight() -> u32 {
    1
}

impl Membership {
    pub fn joined(user_id: UserId, demos_id: DemosId, joined_at: Timestamp) -> Self {
        Self {
            user_id,
            demos_id,
            joined_at,
            tier: Tier::Member,
            sanctioned: false,
            contribution: 0,
            enfranchised_at: None,
            granted_weight: 1,
        }
    }

    pub fn is_voter(&self) -> bool {
        self.tier == Tier::Voter
    }

    /// Whether this member may currently exercise the franchise: an enfranchised
    /// voter who is **not** under an active sanction. A sanction disqualifies from
    /// the franchise (see [`sanctioned`](Self::sanctioned)), so every governance
    /// action — casting a proposal ballot, being empanelled on a jury, voting a
    /// verdict — must gate on this, not on the bare [`is_voter`](Self::is_voter)
    /// tier (which a convicted member retains until they re-qualify).
    pub fn is_franchised(&self) -> bool {
        self.is_voter() && !self.sanctioned
    }

    /// Whole days this user has been a member of the demos.
    pub fn membership_age_days(&self, now: Timestamp) -> i64 {
        now.days_since(self.joined_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voter() -> Membership {
        let mut m = Membership::joined(UserId(1), DemosId(1), Timestamp(0));
        m.tier = Tier::Voter;
        m
    }

    #[test]
    fn a_clean_voter_is_franchised() {
        assert!(voter().is_franchised());
    }

    #[test]
    fn a_sanctioned_voter_keeps_the_tier_but_loses_the_franchise() {
        let mut m = voter();
        m.sanctioned = true;
        assert!(
            m.is_voter(),
            "the Voter tier is retained until re-qualification"
        );
        assert!(
            !m.is_franchised(),
            "but a sanction disqualifies from the franchise"
        );
    }

    #[test]
    fn a_non_voter_is_never_franchised() {
        let mut m = Membership::joined(UserId(1), DemosId(1), Timestamp(0));
        assert_eq!(m.tier, Tier::Member);
        assert!(!m.is_franchised());
        m.tier = Tier::Lurker;
        assert!(!m.is_franchised());
    }
}
