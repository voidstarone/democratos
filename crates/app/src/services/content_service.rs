//! Content use-cases: creating posts (with the automatic bot and NSFW checks),
//! commenting, listing, and casting/reading up-down votes on posts and comments.
//! Owns the content and vote ports and leans on four peers — the account service
//! to verify a signed post vote, the notification service to fan out mention
//! alerts, the metrics service to refresh cached popularity, and the moderation
//! service for the posting guards and automatic flagging.

use std::collections::HashSet;
use std::sync::Arc;

use domain::{
    bot_score, is_likely_bot, is_nsfw_text, BotSignals, Comment, CommentId, DemosId, Media, Post,
    PostId, ReportReason, ReportTarget, Timestamp, UserId,
};

use crate::identity::post_vote_message::post_vote_message;
use crate::{
    Clock, CommentStore, CommentVoteStore, CreatePostError, DemosStore, MediaVerdict,
    MemberActionError, NsfwScanner, PostStore, PostVoteStore, Result, StoreError, UserStore,
    VotePostError,
};

use super::account_service::AccountService;
use super::metrics_service::MetricsService;
use super::moderation_service::ModerationService;
use super::notification_service::NotificationService;

/// Content use-cases, over just the content and vote ports plus the account,
/// notification, metrics, and moderation peers.
#[derive(Clone)]
pub struct ContentService {
    posts: Arc<dyn PostStore>,
    comments: Arc<dyn CommentStore>,
    post_votes: Arc<dyn PostVoteStore>,
    comment_votes: Arc<dyn CommentVoteStore>,
    nsfw_scanner: Arc<dyn NsfwScanner>,
    demoi: Arc<dyn DemosStore>,
    users: Arc<dyn UserStore>,
    clock: Arc<dyn Clock>,
    account: Arc<AccountService>,
    notification: Arc<NotificationService>,
    metrics: Arc<MetricsService>,
    moderation: Arc<ModerationService>,
}

impl ContentService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        posts: Arc<dyn PostStore>,
        comments: Arc<dyn CommentStore>,
        post_votes: Arc<dyn PostVoteStore>,
        comment_votes: Arc<dyn CommentVoteStore>,
        nsfw_scanner: Arc<dyn NsfwScanner>,
        demoi: Arc<dyn DemosStore>,
        users: Arc<dyn UserStore>,
        clock: Arc<dyn Clock>,
        account: Arc<AccountService>,
        notification: Arc<NotificationService>,
        metrics: Arc<MetricsService>,
        moderation: Arc<ModerationService>,
    ) -> Self {
        Self {
            posts,
            comments,
            post_votes,
            comment_votes,
            nsfw_scanner,
            demoi,
            users,
            clock,
            account,
            notification,
            metrics,
            moderation,
        }
    }

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
        self.moderation
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
        self.notification
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
            self.moderation
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
        self.moderation
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
        self.metrics
            .recompute_popularity(author, post.demos_id)
            .await?;
        self.run_bot_check(author, post.demos_id, now).await?;
        // Ping anyone named in the reply (their opt-in is checked inside).
        self.notification
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
        self.moderation
            .require_unsanctioned_member(user, post.demos_id)
            .await?;
        // Signed by the acting user (the client signs the *resolved* direction it
        // is applying, which it can compute from the vote state it rendered), so a
        // relaying node can't forge or flip a member's post vote. Verified here on
        // the authoritative owner, for both local and forwarded votes.
        self.account
            .verify_user_action(user, &post_vote_message(post_id.0, dir), sig)
            .await?;
        self.post_votes.set(post_id, user, dir).await?;
        // The post author's popularity just changed; refresh their cached score.
        self.metrics
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
        self.moderation
            .require_unsanctioned_member(user, post.demos_id)
            .await?;
        self.comment_votes.set(comment_id, user, dir).await?;
        // The comment author's popularity just changed; refresh their cache.
        self.metrics
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

    /// Assemble behavioural signals and, if they cross the threshold, file an
    /// automatic bot report (unless one is already open for this user).
    async fn run_bot_check(&self, author: UserId, demos: DemosId, now: Timestamp) -> Result<()> {
        let signals = self.bot_signals(author, demos, now).await?;
        if !is_likely_bot(&signals) {
            return Ok(());
        }
        // Folds into any open report already on this user; `add_flag` ignores a
        // duplicate Bot flag if the detector fires again.
        self.moderation
            .file_or_merge_flag(
                demos,
                None,
                ReportTarget::User(author),
                ReportReason::Bot,
                &format!("automatic: bot score {}", bot_score(&signals)),
                now,
            )
            .await?;
        Ok(())
    }

    pub async fn bot_signals(
        &self,
        author: UserId,
        demos: DemosId,
        now: Timestamp,
    ) -> Result<BotSignals> {
        let user = self.users.get(author).await?.ok_or(StoreError::NotFound)?;
        let posts = self.posts.list_by_author(demos, author).await?;
        let hour_ago = Timestamp(now.0 - 3600);

        let recent_posts = posts.iter().filter(|p| p.created_at >= hour_ago).count() as u32;
        let recent_comments = self
            .comments
            .count_by_author_since(author, hour_ago)
            .await? as u32;

        let distinct: HashSet<(String, String)> = posts
            .iter()
            .map(|p| (p.title.clone(), p.text_content()))
            .collect();
        let duplicate_actions = (posts.len() as u32).saturating_sub(distinct.len() as u32);
        let demos_spammed = self.posts.distinct_demos_by_author(author).await? as u32;

        Ok(BotSignals {
            account_age_days: user.account_age_days(now),
            actions_last_hour: recent_posts + recent_comments,
            duplicate_actions,
            demos_spammed,
        })
    }
}
