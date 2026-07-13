//! Persistence for comments.

use async_trait::async_trait;

use domain::{Comment, CommentId, PostId, Timestamp, UserId};

use crate::Result;

#[async_trait]
pub trait CommentStore: Send + Sync {
    async fn create(
        &self,
        post: PostId,
        author: UserId,
        parent: Option<CommentId>,
        body: &str,
        at: Timestamp,
    ) -> Result<Comment>;
    async fn get(&self, id: CommentId) -> Result<Option<Comment>>;
    async fn set_removed(&self, id: CommentId, removed: bool) -> Result<()>;
    /// Hide (or un-hide) a comment pending sensitive-content review.
    async fn set_pending_review(&self, id: CommentId, pending: bool) -> Result<()>;
    async fn list_for_post(&self, post: PostId) -> Result<Vec<Comment>>;
    async fn count_by_author_since(&self, author: UserId, since: Timestamp) -> Result<u64>;
    /// Every comment this user has authored, across all posts. Backs the
    /// per-community popularity metric (net upvotes on their comments).
    async fn list_by_author(&self, author: UserId) -> Result<Vec<Comment>>;
}
