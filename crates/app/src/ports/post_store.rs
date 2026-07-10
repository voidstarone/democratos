//! Persistence for posts.

use async_trait::async_trait;

use domain::{DemosId, Media, Post, PostId, Timestamp, UserId};

use crate::Result;

#[async_trait]
pub trait PostStore: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        demos: DemosId,
        author: UserId,
        title: &str,
        body: &str,
        media: Vec<Media>,
        tags: Vec<String>,
        at: Timestamp,
    ) -> Result<Post>;
    async fn get(&self, id: PostId) -> Result<Option<Post>>;
    async fn set_removed(&self, id: PostId, removed: bool) -> Result<()>;
    /// Flag (or unflag) a post as NSFW.
    async fn set_is_nsfw(&self, id: PostId, is_nsfw: bool) -> Result<()>;
    async fn list(&self, demos: DemosId) -> Result<Vec<Post>>;
    async fn list_by_author(&self, demos: DemosId, author: UserId) -> Result<Vec<Post>>;
    /// Every post across all communities. Backs site-wide search.
    async fn list_all(&self) -> Result<Vec<Post>>;
    /// Distinct demos this author has posted in — a cross-posting signal.
    async fn distinct_demos_by_author(&self, author: UserId) -> Result<u64>;
}
