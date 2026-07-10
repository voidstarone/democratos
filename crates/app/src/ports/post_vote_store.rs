//! Persistence for up/down votes on posts.

use async_trait::async_trait;

use domain::{PostId, UserId};

use crate::Result;

/// Up/down votes on posts (distinct from governance proposal votes). One vote
/// per member per post; the net score (upvotes − downvotes) drives the home feed.
#[async_trait]
pub trait PostVoteStore: Send + Sync {
    /// Set a member's vote on a post: `Some(true)` = up, `Some(false)` = down,
    /// `None` = clear. Replaces any prior vote by the same user on the post.
    async fn set(&self, post: PostId, user: UserId, dir: Option<bool>) -> Result<()>;
    /// This user's current vote on the post, if any.
    async fn get(&self, post: PostId, user: UserId) -> Result<Option<bool>>;
    /// Net score: upvotes minus downvotes.
    async fn score(&self, post: PostId) -> Result<i64>;
    /// Total number of votes on record. A cheap version stamp: the recommender
    /// only rebuilds its model when this changes, so the read path never has to
    /// snapshot the full vote history.
    async fn vote_count(&self) -> Result<u64>;
    /// Every vote across every post, as `(post, user, up)`. The bulk read that
    /// backs a recommender rebuild — taken only when [`vote_count`](Self::vote_count)
    /// shows the model is stale, never on a plain read.
    async fn all_votes(&self) -> Result<Vec<(PostId, UserId, bool)>>;
    /// The posts this user *upvoted* — the positive signal seeding their
    /// recommendations.
    async fn liked_by(&self, user: UserId) -> Result<Vec<PostId>>;
    /// Every post this user has voted on (up *or* down) — excluded from their
    /// recommendations so the feed only surfaces something new.
    async fn voted_by(&self, user: UserId) -> Result<Vec<PostId>>;
}
