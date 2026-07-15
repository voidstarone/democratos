//! A user's membership record within a demos.

use serde::{Deserialize, Serialize};

use crate::{DemosId, Tier, Timestamp, UserId, MAX_SANCTION_DAYS};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Membership {
    pub user_id: UserId,
    pub demos_id: DemosId,
    pub joined_at: Timestamp,
    pub tier: Tier,
    /// When the member's current sanction lifts, or `None` if unsanctioned. A
    /// sanction disqualifies from the franchise **only until this time passes** —
    /// there is no permanent ban. Applied only via [`sanction_for`](Self::sanction_for),
    /// which caps the term at [`MAX_SANCTION_DAYS`], so no value here is ever more
    /// than 18 years out. `#[serde(default)]` loads older datasets as unsanctioned,
    /// which also lifts any legacy permanent (`sanctioned: true`) ban on upgrade.
    #[serde(default)]
    pub sanctioned_until: Option<Timestamp>,
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
            sanctioned_until: None,
            contribution: 0,
            enfranchised_at: None,
            granted_weight: 1,
        }
    }

    pub fn is_voter(&self) -> bool {
        self.tier == Tier::Voter
    }

    /// Whether an active sanction is in force at `now` — i.e. the ban has not yet
    /// lapsed. A sanction always expires, so this is time-relative by design.
    pub fn is_sanctioned(&self, now: Timestamp) -> bool {
        self.sanctioned_until.is_some_and(|until| now.0 < until.0)
    }

    /// Sanction this member for `days` from `now`, **clamped to
    /// [`MAX_SANCTION_DAYS`]** (18 years). This is the ONLY way a sanction is set,
    /// which is what makes a permanent ban impossible: no caller can express a term
    /// the domain won't cap. Extends (never shortens) any sanction already longer.
    pub fn sanction_for(&mut self, now: Timestamp, days: u32) {
        let capped = days.min(MAX_SANCTION_DAYS) as i64;
        let until = now.plus_days(capped);
        // Take the later of the two so a fresh, longer sanction can't be undercut
        // by a shorter one still running — but the cap still bounds it.
        let until = match self.sanctioned_until {
            Some(existing) if existing.0 > until.0 => existing,
            _ => until,
        };
        self.sanctioned_until = Some(until);
    }

    /// Lift any sanction (e.g. a successful appeal).
    pub fn clear_sanction(&mut self) {
        self.sanctioned_until = None;
    }

    /// Whether this member may currently exercise the franchise: an enfranchised
    /// voter who is **not** under an active sanction. A sanction disqualifies from
    /// the franchise (see [`sanctioned`](Self::sanctioned)), so every governance
    /// action — casting a proposal ballot, being empanelled on a jury, voting a
    /// verdict — must gate on this, not on the bare [`is_voter`](Self::is_voter)
    /// tier (which a convicted member retains until they re-qualify).
    pub fn is_franchised(&self, now: Timestamp) -> bool {
        self.is_voter() && !self.is_sanctioned(now)
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

    const DAY: i64 = Timestamp::SECONDS_PER_DAY;

    #[test]
    fn a_clean_voter_is_franchised() {
        assert!(voter().is_franchised(Timestamp(0)));
    }

    #[test]
    fn a_sanctioned_voter_keeps_the_tier_but_loses_the_franchise_until_it_lapses() {
        let now = Timestamp(100 * DAY);
        let mut m = voter();
        m.sanction_for(now, 30);
        assert!(
            m.is_voter(),
            "the Voter tier is retained until re-qualification"
        );
        assert!(
            !m.is_franchised(now),
            "a sanction disqualifies from the franchise while it runs"
        );
        // ...but it always lapses — there are no permanent bans.
        assert!(m.is_franchised(Timestamp((100 + 31) * DAY)));
    }

    #[test]
    fn a_sanction_is_capped_at_the_platform_maximum() {
        let now = Timestamp(0);
        let mut m = voter();
        m.sanction_for(now, u32::MAX); // ask for forever
        let until = m.sanctioned_until.unwrap();
        assert_eq!(until, now.plus_days(MAX_SANCTION_DAYS as i64));
        // Still sanctioned right up to the cap...
        assert!(m.is_sanctioned(now.plus_days(MAX_SANCTION_DAYS as i64 - 1)));
        // ...but it MUST lapse after: no permaban, even asking for u32::MAX.
        assert!(!m.is_sanctioned(now.plus_days(MAX_SANCTION_DAYS as i64 + 1)));
        assert!(m.is_franchised(now.plus_days(MAX_SANCTION_DAYS as i64 + 1)));
    }

    #[test]
    fn a_non_voter_is_never_franchised() {
        let mut m = Membership::joined(UserId(1), DemosId(1), Timestamp(0));
        assert_eq!(m.tier, Tier::Member);
        assert!(!m.is_franchised(Timestamp(0)));
        m.tier = Tier::Lurker;
        assert!(!m.is_franchised(Timestamp(0)));
    }
}
