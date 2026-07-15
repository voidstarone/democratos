

use domain::{
    Comment, CommentId,
    DemosId, Media, Post,
    PostId,
    ReportReason, ReportTarget, Timestamp, UserId,
};

use domain::is_nsfw_text;

use crate::identity::post_vote_message::post_vote_message;
use crate::{MediaVerdict};
use crate::{
    CreatePostError, MemberActionError, Result, StoreError,
    VotePostError,
};


use super::services::Services;

impl Services {
    /// Create a post. The author must be a member in good standing. After
    /// posting, the bot detector runs and may file an automatic report.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_post(
        &self,
        author: UserId,
        demos: DemosId,
        title: &str,
        body: &str,
        media: Vec<Media>,
        tags: Vec<String>,
    ) -> Result<Post, CreatePostError> {
        self.moderation_service()
            .require_can_post(author, demos)
            .await?;
        let now = self.clock.now();
        let mut post = self
            .posts
            .create(demos, author, title, body, media, tags, now)
            .await?;
        self.run_bot_check(author, demos, now).await?;
        post.is_nsfw = self.run_nsfw_check(&post, now).await?;
        // Ping anyone named in the title or body (their opt-in is checked inside).
        self.notification_service()
            .notify_mentions(author, &format!("{title} {body}"), post.id, None)
            .await?;
        Ok(post)
    }

    /// Flag a post NSFW when its text, tags, or media look explicit. In a
    /// community that has *voted to forbid* NSFW, a flagged post also auto-files
    /// a report for a jury — "the machine flags; the demos judges". It never
    /// removes the post itself. Returns whether it was flagged.
    async fn run_nsfw_check(&self, post: &Post, now: Timestamp) -> Result<bool> {
        let text = format!(
            "{} {} {}",
            post.title,
            post.text_content(),
            post.tags.join(" ")
        );
        let mut is_nsfw = is_nsfw_text(&text) || post.tags.iter().any(|t| t == "nsfw");
        if !is_nsfw {
            // Scan each attachment; any explicit item flags the whole post.
            for m in &post.media {
                let verdict = self
                    .nsfw_scanner
                    .scan_media(&m.url, &m.caption, m.kind_label())
                    .await?;
                if verdict == MediaVerdict::Nsfw {
                    is_nsfw = true;
                    break;
                }
            }
        }
        if !is_nsfw {
            return Ok(false);
        }
        self.posts.set_is_nsfw(post.id, true).await?;

        let demos = self
            .demoi
            .get(post.demos_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        if !demos.allows_nsfw {
            // Folds into any open report already on this post (and `add_flag`
            // ignores a duplicate NSFW flag if the scanner re-runs).
            self.moderation_service()
                .file_or_merge_flag(
                    post.demos_id,
                    None,
                    ReportTarget::Post(post.id),
                    ReportReason::Nsfw,
                    "automatic: NSFW content in a community that forbids it",
                    now,
                )
                .await?;
        }
        Ok(true)
    }

    /// Reply to a post (or, with `parent`, to another comment).
    pub async fn comment(
        &self,
        author: UserId,
        post_id: PostId,
        parent: Option<CommentId>,
        body: &str,
    ) -> Result<Comment, MemberActionError> {
        let post = self.posts.get(post_id).await?.ok_or(StoreError::NotFound)?;
        self.moderation_service()
            .require_unsanctioned_member(author, post.demos_id)
            .await?;
        let now = self.clock.now();
        let comment = self
            .comments
            .create(post_id, author, parent, body, now)
            .await?;
        // Every comment starts with its author's own upvote, so each begins at a
        // net score of 1 — the same baseline for everyone. `set` is idempotent per
        // (comment, voter), so this never double-counts.
        self.comment_votes
            .set(comment.id, author, Some(true))
            .await?;
        self.metrics_service()
            .recompute_popularity(author, post.demos_id)
            .await?;
        self.run_bot_check(author, post.demos_id, now).await?;
        // Ping anyone named in the reply (their opt-in is checked inside).
        self.notification_service()
            .notify_mentions(author, body, post_id, Some(comment.id))
            .await?;
        Ok(comment)
    }

    pub async fn list_posts(&self, demos: DemosId) -> Result<Vec<Post>> {
        self.posts.list(demos).await
    }

    pub async fn comments_for(&self, post: PostId) -> Result<Vec<Comment>> {
        self.comments.list_for_post(post).await
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
        let post = self.posts.get(post_id).await?.ok_or(StoreError::NotFound)?;
        self.moderation_service()
            .require_unsanctioned_member(user, post.demos_id)
            .await?;
        // Signed by the acting user (the client signs the *resolved* direction it
        // is applying, which it can compute from the vote state it rendered), so a
        // relaying node can't forge or flip a member's post vote. Verified here on
        // the authoritative owner, for both local and forwarded votes.
        self.account_service()
            .verify_user_action(user, &post_vote_message(post_id.0, dir), sig)
            .await?;
        self.post_votes.set(post_id, user, dir).await?;
        // The post author's popularity just changed; refresh their cached score.
        self.metrics_service()
            .recompute_popularity(post.author, post.demos_id)
            .await?;
        Ok(self.post_votes.score(post_id).await?)
    }

    pub async fn post_score(&self, post: PostId) -> Result<i64> {
        self.post_votes.score(post).await
    }

    pub async fn user_post_vote(&self, post: PostId, user: UserId) -> Result<Option<bool>> {
        self.post_votes.get(post, user).await
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
        let comment = self
            .comments
            .get(comment_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        let post = self
            .posts
            .get(comment.post_id)
            .await?
            .ok_or(StoreError::NotFound)?;
        self.moderation_service()
            .require_unsanctioned_member(user, post.demos_id)
            .await?;
        self.comment_votes.set(comment_id, user, dir).await?;
        // The comment author's popularity just changed; refresh their cache.
        self.metrics_service()
            .recompute_popularity(comment.author, post.demos_id)
            .await?;
        Ok(self.comment_votes.score(comment_id).await?)
    }

    pub async fn comment_score(&self, comment: CommentId) -> Result<i64> {
        self.comment_votes.score(comment).await
    }

    pub async fn user_comment_vote(
        &self,
        comment: CommentId,
        user: UserId,
    ) -> Result<Option<bool>> {
        self.comment_votes.get(comment, user).await
    }
}
