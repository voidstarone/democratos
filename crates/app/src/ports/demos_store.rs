//! Persistence for communities (a *demos*).

use async_trait::async_trait;

use domain::{
    Demos, DemosId, FranchiseCriteria, JurySizing, PostingPolicy, Timestamp, UserId, VoteWeighting,
    WeightingScope,
};

use crate::Result;

#[async_trait]
pub trait DemosStore: Send + Sync {
    async fn create(
        &self,
        slug: &str,
        name: &str,
        founder: UserId,
        tags: Vec<String>,
        created_at: Timestamp,
    ) -> Result<Demos>;
    async fn get(&self, id: DemosId) -> Result<Option<Demos>>;
    async fn by_slug(&self, slug: &str) -> Result<Option<Demos>>;
    /// Communities carrying the exact tag `tag`, newest first. Backed by the
    /// pipe-wrapped tag index (`|tag1|tag2|`) so this is an indexed lookup, not a
    /// scan of every community. `tag` must already be normalized (`[a-z0-9-]`).
    async fn by_tag(&self, tag: &str) -> Result<Vec<Demos>>;
    async fn update_criteria(&self, id: DemosId, criteria: FranchiseCriteria) -> Result<()>;
    /// Set whether the demos permits NSFW content (changed by a passed
    /// `SetNsfwPolicy` proposal).
    async fn set_allows_nsfw(&self, id: DemosId, allows_nsfw: bool) -> Result<()>;
    /// Set how the demos sizes report juries (changed by `SetJurySizing`).
    async fn set_jury_sizing(&self, id: DemosId, sizing: JurySizing) -> Result<()>;
    /// Set the demos's vote-weighting scheme (changed by `SetVoteWeighting`).
    async fn set_vote_weighting(&self, id: DemosId, scheme: VoteWeighting) -> Result<()>;
    /// Set which decisions vote-weighting applies to (changed by `SetWeightingScope`).
    async fn set_weighting_scope(&self, id: DemosId, scope: WeightingScope) -> Result<()>;
    /// Set who may post here (changed by `SetPostingPolicy`).
    async fn set_posting_policy(&self, id: DemosId, policy: PostingPolicy) -> Result<()>;
    /// Set the community's ban ceiling in days (changed by `SetMaxSanction`).
    /// Callers pass an already-clamped value; the store just records it.
    async fn set_max_sanction(&self, id: DemosId, days: u32) -> Result<()>;
    async fn list(&self) -> Result<Vec<Demos>>;
}
