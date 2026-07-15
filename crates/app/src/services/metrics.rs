
use std::collections::HashSet;

use domain::{
    bot_score, is_likely_bot, BotSignals,
    DemosId,
    ReportReason, ReportTarget, Timestamp, UserId,
};


use crate::{Result, StoreError};

use super::member_metrics::MemberMetrics;

use super::metrics_service::MetricsService;
use super::services::Services;

impl Services {
    /// Build the extracted [`MetricsService`] from the ports this aggregator still
    /// holds. Cheap — `Arc` clones only — so callers construct one per call rather
    /// than storing a field (which would break every `Services { … }` literal).
    /// Removed once all call sites inject `MetricsService` directly.
    pub(super) fn metrics_service(&self) -> MetricsService {
        MetricsService::new(
            self.posts.clone(),
            self.comments.clone(),
            self.post_votes.clone(),
            self.comment_votes.clone(),
            self.memberships.clone(),
        )
    }

    /// Compute a member's engagement metrics in one community: net upvotes on
    /// their posts and comments here (plus the counts). Popularity — the sum —
    /// is what gates the franchise and posting policy.
    pub async fn member_metrics(&self, user: UserId, demos: DemosId) -> Result<MemberMetrics> {
        self.metrics_service().member_metrics(user, demos).await
    }

    /// Assemble behavioural signals and, if they cross the threshold, file an
    /// automatic bot report (unless one is already open for this user).
    pub(super) async fn run_bot_check(&self, author: UserId, demos: DemosId, now: Timestamp) -> Result<()> {
        let signals = self.bot_signals(author, demos, now).await?;
        if !is_likely_bot(&signals) {
            return Ok(());
        }
        // Folds into any open report already on this user; `add_flag` ignores a
        // duplicate Bot flag if the detector fires again.
        self.file_or_merge_flag(
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
