use domain::{BotSignals, DemosId, Timestamp, UserId};

use crate::Result;

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

    /// Assemble a user's behavioural bot signals. The logic now lives in
    /// [`ContentService`](super::content_service::ContentService) alongside the bot
    /// check that consumes it; this delegator keeps `services.bot_signals()`
    /// working for call sites not yet migrated off the `Services` aggregator.
    pub async fn bot_signals(
        &self,
        author: UserId,
        demos: DemosId,
        now: Timestamp,
    ) -> Result<BotSignals> {
        self.content_service().bot_signals(author, demos, now).await
    }
}
