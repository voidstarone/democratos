//! Facade delegators for feed use-cases. The logic now lives in
//! [`FeedService`](super::feed_service::FeedService); these thin methods keep
//! `services.top_posts()`/`feed()`/recommendation handles working for call sites
//! not yet migrated off the `Services` aggregator.

use domain::UserId;

use crate::{RecommendFeed, RefreshRecommendations, Result};

use super::feed_item::FeedItem;
use super::feed_service::FeedService;
use super::services::Services;

impl Services {
    pub(super) fn feed_service(&self) -> FeedService {
        FeedService::new(
            self.demoi.clone(),
            self.posts.clone(),
            self.post_votes.clone(),
            self.memberships.clone(),
            self.recommender.clone(),
        )
    }

    /// The site-wide "top" feed. See [`FeedService::top_posts`].
    pub async fn top_posts(&self) -> Result<Vec<FeedItem>> {
        self.feed_service().top_posts().await
    }

    /// The personalized home feed. See [`FeedService::feed`].
    pub async fn feed(&self, user: UserId) -> Result<Vec<FeedItem>> {
        self.feed_service().feed(user).await
    }

    /// The recommendation read use case.
    pub fn recommend_feed(&self) -> RecommendFeed {
        self.feed_service().recommend_feed()
    }

    /// The recommendation model-refresh use case.
    pub fn refresh_recommendations(&self) -> RefreshRecommendations {
        self.feed_service().refresh_recommendations()
    }
}
