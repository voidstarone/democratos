//! Community rules — the conduct/content policy a demos sets for itself.
//!
//! Rules are amended through the governance system ([`crate::ProposalKind::AddRule`]
//! / [`crate::ProposalKind::RemoveRule`], decision class
//! [`crate::DecisionClass::RuleChange`]). Unlike the franchise criteria, rules
//! may be set even in the Seed phase, so a founding community can establish its
//! rulebook from day one.

use serde::{Deserialize, Serialize};

use crate::{DemosId, RuleId};
use crate::time::Timestamp;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub id: RuleId,
    pub demos_id: DemosId,
    pub text: String,
    pub enacted_at: Timestamp,
    /// Repealed rules are kept for the record but no longer in force.
    pub active: bool,
    /// The ban term, in days, a jury conviction for breaking *this* rule carries
    /// — set by the community when it votes the rule in (see
    /// [`crate::ProposalKind::AddRule`]) and clamped to the demos ceiling at
    /// enactment. `0` means "unspecified": fall back to the community ceiling. This
    /// is what ties a ban's length to the specific rule broken, decided by the
    /// voters ahead of any trial. `#[serde(default)]` reads older rules as `0`,
    /// preserving today's behaviour (they fall back to the community ceiling).
    #[serde(default)]
    pub sanction_days: u32,
}

impl Rule {
    pub fn new(
        id: RuleId,
        demos_id: DemosId,
        text: impl Into<String>,
        sanction_days: u32,
        enacted_at: Timestamp,
    ) -> Self {
        Self {
            id,
            demos_id,
            text: text.into(),
            enacted_at,
            active: true,
            sanction_days,
        }
    }

    /// The ban term this rule carries against a `community_ceiling` (a demos'
    /// [`crate::Demos::ban_ceiling_days`]). An unspecified term (`0`) inherits the
    /// ceiling; a set term is still clamped to it, so a rule can never punish
    /// beyond what the community has voted to allow.
    pub fn term_days(&self, community_ceiling: u32) -> u32 {
        if self.sanction_days == 0 {
            community_ceiling
        } else {
            self.sanction_days.min(community_ceiling)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(sanction_days: u32) -> Rule {
        Rule::new(RuleId(1), DemosId(1), "no spam", sanction_days, Timestamp(0))
    }

    #[test]
    fn an_unspecified_term_inherits_the_community_ceiling() {
        assert_eq!(rule(0).term_days(30), 30);
    }

    #[test]
    fn a_set_term_is_used_as_is_when_within_the_ceiling() {
        assert_eq!(rule(7).term_days(30), 7);
    }

    #[test]
    fn a_term_over_the_ceiling_is_clamped_to_it() {
        // The rule was voted at 90 days, but the community later lowered its
        // ceiling to 30: the ceiling wins, so the rule can't outrun the vote.
        assert_eq!(rule(90).term_days(30), 30);
    }
}
