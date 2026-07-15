//! Facade delegators for content use-cases. The logic now lives in
//! [`ContentService`](super::content_service::ContentService); these thin methods
//! keep `services.create_post()` and friends working for call sites not yet
//! migrated off the `Services` aggregator.

use std::sync::Arc;

use domain::{Comment, CommentId, DemosId, Media, Post, PostId, UserId};

use crate::{CreatePostError, MemberActionError, Result, VotePostError};

use super::content_service::ContentService;
use super::services::Services;

impl Services {
    /// Build the extracted [`ContentService`] from the ports this aggregator still
    /// holds, wiring its account, notification, metrics, and moderation peers
    /// inline. Cheap — `Arc` clones only — so delegators construct one per call
    /// rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `ContentService` directly.
    pub(super) fn content_service(&self) -> ContentService {
        ContentService::new(
            self.posts.clone(),
            self.comments.clone(),
            self.post_votes.clone(),
            self.comment_votes.clone(),
            self.nsfw_scanner.clone(),
            self.demoi.clone(),
            self.users.clone(),
            self.clock.clone(),
            Arc::new(self.account_service()),
            Arc::new(self.notification_service()),
            Arc::new(self.metrics_service()),
            Arc::new(self.moderation_service()),
        )
    }

    /// Create a post. The author must be a member in good standing. After
    /// posting, the bot detector runs and may file an automatic report.
    pub async fn create_post(
        &self,
        author: UserId,
        demos: DemosId,
        title: &str,
        body: &str,
        media: Vec<Media>,
        tags: Vec<String>,
    ) -> Result<Post, CreatePostError> {
        self.content_service()
            .create_post(author, demos, title, body, media, tags)
            .await
    }

    /// Reply to a post (or, with `parent`, to another comment).
    pub async fn comment(
        &self,
        author: UserId,
        post_id: PostId,
        parent: Option<CommentId>,
        body: &str,
    ) -> Result<Comment, MemberActionError> {
        self.content_service()
            .comment(author, post_id, parent, body)
            .await
    }

    pub async fn list_posts(&self, demos: DemosId) -> Result<Vec<Post>> {
        self.content_service().list_posts(demos).await
    }

    pub async fn comments_for(&self, post: PostId) -> Result<Vec<Comment>> {
        self.content_service().comments_for(post).await
    }

    /// Cast (or toggle/clear) a member's up/down vote on a post. Only members in
    /// good standing of the post's community may vote. Returns the new net score.
    pub async fn vote_post(
        &self,
        post_id: PostId,
        user: UserId,
        dir: Option<bool>,
        sig: Option<&str>,
    ) -> Result<i64, VotePostError> {
        self.content_service()
            .vote_post(post_id, user, dir, sig)
            .await
    }

    pub async fn post_score(&self, post: PostId) -> Result<i64> {
        self.content_service().post_score(post).await
    }

    pub async fn user_post_vote(&self, post: PostId, user: UserId) -> Result<Option<bool>> {
        self.content_service().user_post_vote(post, user).await
    }

    /// Cast (or toggle/clear) a member's up/down vote on a comment. Only members
    /// in good standing of the comment's community may vote. Returns the new net
    /// score.
    pub async fn vote_comment(
        &self,
        comment_id: CommentId,
        user: UserId,
        dir: Option<bool>,
    ) -> Result<i64, MemberActionError> {
        self.content_service()
            .vote_comment(comment_id, user, dir)
            .await
    }

    pub async fn comment_score(&self, comment: CommentId) -> Result<i64> {
        self.content_service().comment_score(comment).await
    }

    pub async fn user_comment_vote(
        &self,
        comment: CommentId,
        user: UserId,
    ) -> Result<Option<bool>> {
        self.content_service().user_comment_vote(comment, user).await
    }
}
