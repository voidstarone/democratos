//! What a proposal seeks to do.

use serde::{Deserialize, Serialize};

use crate::{
    DecisionClass, FranchiseCriteria, JurySizing, RuleId, UserId, VoteWeighting, WeightingScope,
};

/// What a proposal seeks to do — and, via [`ProposalKind::decision_class`], how
/// hard it should be to pass.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ProposalKind {
    /// Remove a post/comment, resolve a report. Routine.
    RemoveContent { target: String },
    /// Ban a user from the demos.
    Ban { user: UserId },
    /// Recall a leader from office.
    Recall { leader: UserId },
    /// Amend the franchise criteria — a constitutional change.
    AmendCriteria { proposed: FranchiseCriteria },
    /// Add a community rule, with the ban term (in days) a conviction for breaking
    /// it carries. The term is clamped to the community ceiling
    /// ([`crate::ProposalKind::SetMaxSanction`]) at enactment; `0` means "inherit
    /// the ceiling". This is how the voters fix a rule's punishment *before* any
    /// jury sits — a ban's length is tied to the specific rule broken, not chosen
    /// case-by-case at trial.
    AddRule { text: String, sanction_days: u32 },
    /// Repeal an existing community rule.
    RemoveRule { rule: RuleId },
    /// Set the community's ceiling on any single ban, in days. Clamped to the
    /// 18-year platform cap ([`crate::MAX_SANCTION_DAYS`]); no community can vote a
    /// permaban. Lowering it re-bounds every rule term and conviction, since those
    /// are clamped to the live ceiling.
    SetMaxSanction { days: u32 },
    /// Set whether this community permits NSFW content. NSFW is allowed-but-gated
    /// by default; passing this with `allows_nsfw: false` makes the demos forbid
    /// it, so detected NSFW posts are auto-reported for a jury.
    SetNsfwPolicy { allows_nsfw: bool },
    /// Change how reports are juried — how many citizens must judge a post or a
    /// comment (see [`JurySizing`]).
    SetJurySizing { sizing: JurySizing },
    /// Change how the demos values its citizens' votes (see [`VoteWeighting`]) —
    /// a constitutional change to the power structure.
    SetVoteWeighting { scheme: VoteWeighting },
    /// Change which decisions vote-weighting applies to (see [`WeightingScope`]).
    SetWeightingScope { scope: WeightingScope },
    /// Grant a specific citizen a vote weight, consulted under the
    /// [`VoteWeighting::ByRole`] scheme. `weight: 1` resets them to an ordinary
    /// citizen.
    GrantVoteWeight { user: UserId, weight: u32 },
    /// Set who may create posts here (see [`crate::PostingPolicy`]).
    SetPostingPolicy { policy: crate::PostingPolicy },
}

impl ProposalKind {
    pub fn decision_class(&self) -> DecisionClass {
        match self {
            ProposalKind::RemoveContent { .. } => DecisionClass::Moderation,
            ProposalKind::Ban { .. }
            | ProposalKind::Recall { .. }
            | ProposalKind::GrantVoteWeight { .. } => DecisionClass::BanOrRecall,
            // Who holds how much power is constitutional.
            ProposalKind::AmendCriteria { .. }
            | ProposalKind::SetVoteWeighting { .. }
            | ProposalKind::SetWeightingScope { .. } => DecisionClass::Constitutional,
            // The ban ceiling is part of the rulebook — it bounds the punishment a
            // rule can carry — so it moves at the same RuleChange bar as the rules
            // themselves, and (like AddRule) a founding demos may set it in Seed.
            // It can only ever be *lowered* below the 18-year platform cap, so
            // there is no permaban risk in letting it pass at the routine bar.
            ProposalKind::AddRule { .. }
            | ProposalKind::RemoveRule { .. }
            | ProposalKind::SetMaxSanction { .. }
            | ProposalKind::SetNsfwPolicy { .. }
            | ProposalKind::SetPostingPolicy { .. }
            | ProposalKind::SetJurySizing { .. } => DecisionClass::RuleChange,
        }
    }
}
