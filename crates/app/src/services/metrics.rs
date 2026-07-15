
use std::collections::HashSet;

use domain::{
    bot_score, is_likely_bot, BotSignals,
    DemosId,
    PostId,
    ReportReason, ReportTarget, Timestamp, UserId,
};


use crate::{Result, StoreError};

use super::member_metrics::MemberMetrics;

use super::services::Services;
use super::vote_value::vote_value;

impl Services {
    /// Compute a member's engagement metrics in one community: net upvotes on
    /// their posts and comments here (plus the counts). Popularity — the sum —
    /// is what gates the franchise and posting policy.
    pub async fn member_metrics(&self, user: UserId, demos: DemosId) -> Result<MemberMetrics> {
        let mut m = MemberMetrics::default();
        for p in self.posts.list_by_author(demos, user).await? {
            if p.removed {
                continue;
            }
            m.posts += 1;
            // Contribution must reflect the *community's* appraisal, not the
            // author's own ballot. Otherwise a user self-qualifies for the
            // franchise (`min_contribution`) and inflates their own
            // `ByContribution` vote weight just by voting on their own content
            // (comments even auto-upvote themselves). Exclude the author's own vote.
            let own = vote_value(self.post_votes.get(p.id, user).await?);
            m.net_post_upvotes += self.post_votes.score(p.id).await? - own;
        }
        // A member's comments in this community = their comments on posts here.
        let here: HashSet<PostId> = self
            .posts
            .list(demos)
            .await?
            .into_iter()
            .map(|p| p.id)
            .collect();
        for c in self.comments.list_by_author(user).await? {
            if c.removed || !here.contains(&c.post_id) {
                continue;
            }
            m.comments += 1;
            let own = vote_value(self.comment_votes.get(c.id, user).await?);
            m.net_comment_upvotes += self.comment_votes.score(c.id).await? - own;
        }
        Ok(m)
    }

    /// Refresh the cached popularity (`Membership::contribution`) for `author` in
    /// `demos` from their current metrics. Called whenever a vote changes the net
    /// score of their content, so eligibility and vote-weighting always read a
    /// current value. A no-op if the author isn't a member.
    pub(super) async fn recompute_popularity(&self, author: UserId, demos: DemosId) -> Result<()> {
        let Some(mut m) = self.memberships.get(author, demos).await? else {
            return Ok(());
        };
        m.contribution = self.member_metrics(author, demos).await?.popularity();
        self.memberships.upsert(m).await
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
