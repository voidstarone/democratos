//! Persistence for up/down votes on comments.

use async_trait::async_trait;

use domain::{CommentId, UserId};

use crate::Result;

/// Up/down votes on comments — the comment counterpart of
/// [`PostVoteStore`](crate::PostVoteStore). One vote per member per comment; the
/// net score feeds a comment's display and the author's community popularity.
#[async_trait]
pub trait CommentVoteStore: Send + Sync {
    /// Set a member's vote: `Some(true)` = up, `Some(false)` = down, `None` =
    /// clear. Replaces any prior vote by the same user on the comment.
    async fn set(&self, comment: CommentId, user: UserId, dir: Option<bool>) -> Result<()>;
    /// This user's current vote on the comment, if any.
    async fn get(&self, comment: CommentId, user: UserId) -> Result<Option<bool>>;
    /// Net score: upvotes minus downvotes.
    async fn score(&self, comment: CommentId) -> Result<i64>;
}
