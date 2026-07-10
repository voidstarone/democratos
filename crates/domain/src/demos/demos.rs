//! The founded demos entity.

use serde::{Deserialize, Serialize};

use crate::{
    DemosId, FranchiseCriteria, JurySizing, PostingPolicy, Timestamp, UserId, VoteWeighting,
    WeightingScope,
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
}

fn allows_nsfw_default() -> bool {
    true
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
        }
    }
}
