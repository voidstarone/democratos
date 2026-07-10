//! What a demos uses to value a citizen's vote.

use serde::{Deserialize, Serialize};

use crate::{Membership, Timestamp, MAX_VOTE_WEIGHT};

/// What a demos uses to value a citizen's vote. Filed under the constitution —
/// changed via [`crate::ProposalKind::SetVoteWeighting`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum VoteWeighting {
    /// One citizen, one vote. The platform default.
    #[default]
    Equal,
    /// Weight grows with recorded contribution, with diminishing returns
    /// (`1 + ⌊√contribution⌋`) so a large score cannot run away.
    ByContribution,
    /// Weight grows with time served in the franchise — one step per full year
    /// since enfranchisement. Resists fresh-account gaming.
    ByTenure,
    /// Weight is the per-member value the community has granted
    /// ([`crate::ProposalKind::GrantVoteWeight`]); ungranted citizens weigh `1`.
    ByRole,
}

impl VoteWeighting {
    /// This member's voting weight under the scheme — always within
    /// `1..=MAX_VOTE_WEIGHT`.
    pub fn weight_of(&self, member: &Membership, now: Timestamp) -> u64 {
        let raw = match self {
            VoteWeighting::Equal => 1,
            VoteWeighting::ByContribution => 1 + (member.contribution.max(0) as u64).isqrt(),
            VoteWeighting::ByTenure => match member.enfranchised_at {
                Some(at) => 1 + (now.days_since(at).max(0) as u64) / 365,
                None => 1,
            },
            VoteWeighting::ByRole => member.granted_weight as u64,
        };
        raw.clamp(1, MAX_VOTE_WEIGHT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DemosId, Tier, UserId};

    const DAY: i64 = Timestamp::SECONDS_PER_DAY;

    fn voter(contribution: i64, enfranchised_days_ago: Option<i64>, granted: u32) -> Membership {
        let now = Timestamp(10_000 * DAY);
        let mut m = Membership::joined(UserId(1), DemosId(1), Timestamp(0));
        m.tier = Tier::Voter;
        m.contribution = contribution;
        m.enfranchised_at = enfranchised_days_ago.map(|d| Timestamp(now.0 - d * DAY));
        m.granted_weight = granted;
        m
    }

    #[test]
    fn equal_is_always_one() {
        let now = Timestamp(10_000 * DAY);
        assert_eq!(
            VoteWeighting::Equal.weight_of(&voter(9_999, Some(3650), 9), now),
            1
        );
    }

    #[test]
    fn contribution_weight_has_diminishing_returns_and_a_floor() {
        let now = Timestamp(10_000 * DAY);
        // √100 = 10 -> weight 11.
        assert_eq!(
            VoteWeighting::ByContribution.weight_of(&voter(100, None, 1), now),
            11
        );
        // Negative/zero contribution never drops below 1.
        assert_eq!(
            VoteWeighting::ByContribution.weight_of(&voter(-5, None, 1), now),
            1
        );
    }

    #[test]
    fn tenure_weight_steps_once_per_year() {
        let now = Timestamp(10_000 * DAY);
        // ~2.5 years served -> 1 + 2 = 3.
        assert_eq!(
            VoteWeighting::ByTenure.weight_of(&voter(0, Some(900), 1), now),
            3
        );
        // Never enfranchised -> 1.
        assert_eq!(
            VoteWeighting::ByTenure.weight_of(&voter(0, None, 1), now),
            1
        );
    }

    #[test]
    fn weights_are_capped() {
        let now = Timestamp(10_000 * DAY);
        // A huge grant is clamped to the ceiling, not honoured literally.
        assert_eq!(
            VoteWeighting::ByRole.weight_of(&voter(0, None, 1_000), now),
            MAX_VOTE_WEIGHT
        );
        // A zero grant still counts as a citizen.
        assert_eq!(VoteWeighting::ByRole.weight_of(&voter(0, None, 0), now), 1);
    }
}
