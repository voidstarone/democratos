//! The founded demos entity.

use serde::{Deserialize, Serialize};

use crate::{
    DemosId, FranchiseCriteria, JurySizing, PostingPolicy, Timestamp, UserId, VoteWeighting,
    WeightingScope, MAX_SANCTION_DAYS,
};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Demos {
    pub id: DemosId,
    pub slug: String,
    pub name: String,
    pub founder_id: UserId,
    pub created_at: Timestamp,
    /// The community's current franchise constitution (amendable by vote).
    pub criteria: FranchiseCriteria,
    /// Whether NSFW content is permitted here. NSFW is allowed-but-gated by
    /// default; a community may *vote to forbid* it
    /// ([`crate::ProposalKind::SetNsfwPolicy`]), after which detected NSFW posts
    /// are auto-reported for a jury. `#[serde(default = "..")]` defaults older
    /// datasets to the permissive baseline.
    #[serde(default = "allows_nsfw_default")]
    pub allows_nsfw: bool,
    /// How this demos sizes the jury that judges a report (amendable by vote).
    /// `#[serde(default)]` gives older datasets the platform-default scaling.
    #[serde(default)]
    pub jury_sizing: JurySizing,
    /// How this demos values its citizens' votes (amendable by vote). Defaults
    /// to one-citizen-one-vote.
    #[serde(default)]
    pub vote_weighting: VoteWeighting,
    /// Which decisions the [`Demos::vote_weighting`] scheme applies to.
    #[serde(default)]
    pub weighting_scope: WeightingScope,
    /// Who may create posts here (amendable by vote — see
    /// [`crate::ProposalKind::SetPostingPolicy`]). Defaults to joined members.
    #[serde(default)]
    pub posting_policy: PostingPolicy,
    /// The community's ceiling on any single ban, in days (amendable by vote — see
    /// [`crate::ProposalKind::SetMaxSanction`]). Every rule's ban term and every
    /// conviction is clamped to this, and this itself can never exceed the 18-year
    /// platform cap ([`MAX_SANCTION_DAYS`]) — so no community can vote a permaban.
    /// `#[serde(default = "..")]` defaults older datasets (and fresh communities)
    /// to the platform cap, i.e. permissive until the demos votes it down.
    #[serde(default = "max_sanction_default")]
    pub max_sanction_days: u32,
    /// Free-form topic tags describing the community, set by the founder when the
    /// founding petition opens. Normalized/deduped (see [`crate::normalize_tags`]);
    /// the store is responsible for making them searchable. `#[serde(default)]`
    /// gives older datasets an empty tag set.
    #[serde(default)]
    pub tags: Vec<String>,
}

fn allows_nsfw_default() -> bool {
    true
}

fn max_sanction_default() -> u32 {
    MAX_SANCTION_DAYS
}

impl Demos {
    pub fn new(
        id: DemosId,
        slug: impl Into<String>,
        name: impl Into<String>,
        founder_id: UserId,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            slug: slug.into(),
            name: name.into(),
            founder_id,
            created_at,
            criteria: FranchiseCriteria::platform_default(),
            allows_nsfw: true,
            jury_sizing: JurySizing::default(),
            vote_weighting: VoteWeighting::default(),
            weighting_scope: WeightingScope::default(),
            posting_policy: PostingPolicy::default(),
            max_sanction_days: MAX_SANCTION_DAYS,
            tags: Vec::new(),
        }
    }

    /// The founding tags, normalized and deduped (empty for older datasets).
    /// Chainable at construction: `Demos::new(..).with_tags(tags)`.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// This community's ban ceiling, never above the platform cap. Use as the
    /// upper bound when enacting a rule's term or applying a conviction.
    pub fn ban_ceiling_days(&self) -> u32 {
        self.max_sanction_days.min(MAX_SANCTION_DAYS)
    }

    /// Clamp a requested ban term to this community's ceiling (which is itself
    /// bounded by the platform cap). The single place a demos-level term is
    /// bounded — so a rule term or a conviction can never outrun the community's
    /// own vote, nor the 18-year platform maximum.
    pub fn cap_sanction_days(&self, days: u32) -> u32 {
        days.min(self.ban_ceiling_days())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demos() -> Demos {
        Demos::new(DemosId(1), "rust", "Rustaceans", UserId(1), Timestamp(0))
    }

    #[test]
    fn a_fresh_demos_defaults_to_the_platform_cap() {
        assert_eq!(demos().max_sanction_days, MAX_SANCTION_DAYS);
        assert_eq!(demos().ban_ceiling_days(), MAX_SANCTION_DAYS);
    }

    #[test]
    fn the_ceiling_never_exceeds_the_platform_cap_even_if_a_field_does() {
        let mut d = demos();
        d.max_sanction_days = u32::MAX; // e.g. a corrupt/hostile record
        assert_eq!(d.ban_ceiling_days(), MAX_SANCTION_DAYS);
        assert_eq!(d.cap_sanction_days(u32::MAX), MAX_SANCTION_DAYS);
    }

    #[test]
    fn a_lowered_ceiling_bounds_requested_terms() {
        let mut d = demos();
        d.max_sanction_days = 30;
        assert_eq!(d.cap_sanction_days(7), 7);
        assert_eq!(d.cap_sanction_days(90), 30);
    }
}
