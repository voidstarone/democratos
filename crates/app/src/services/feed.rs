

use domain::{
    DemosId, UserId,
};

use domain::feed_threshold;

use crate::{RecommendFeed, RefreshRecommendations};
use crate::Result;

use super::feed_item::FeedItem;

use super::services::Services;

/// How many posts the site-wide "top" feed shows.
const TOP_FEED_LIMIT: usize = 50;

impl Services {
    /// The site-wide "top" feed: the most popular non-removed posts across
    /// **every** community, sorted by net score (desc) then recency (desc) and
    /// capped at [`TOP_FEED_LIMIT`]. Unlike [`feed`](Self::feed) it needs no
    /// membership and applies no per-community threshold — it's a global
    /// leaderboard, available to everyone.
    pub async fn top_posts(&self) -> Result<Vec<FeedItem>> {
        let slugs: std::collections::HashMap<DemosId, String> = self
            .demoi
            .list()
            .await?
            .into_iter()
            .map(|d| (d.id, d.slug))
            .collect();
        let mut items: Vec<FeedItem> = Vec::new();
        for post in self.posts.list_all().await? {
            if post.removed || post.pending_review {
                continue;
            }
            let score = self.post_votes.score(post.id).await?;
            let community_slug = slugs.get(&post.demos_id).cloned().unwrap_or_default();
            items.push(FeedItem {
                post,
                score,
                community_slug,
            });
        }
        items.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(b.post.created_at.0.cmp(&a.post.created_at.0))
        });
        items.truncate(TOP_FEED_LIMIT);
        Ok(items)
    }

    /// The personalized home feed: across every community the user has joined,
    /// the non-removed posts whose net score clears that community's
    /// [`feed_threshold`], sorted by score (desc) then recency (desc).
    pub async fn feed(&self, user: UserId) -> Result<Vec<FeedItem>> {
        let mut items: Vec<FeedItem> = Vec::new();
        for membership in self.memberships.list_for_user(user).await? {
            let demos = match self.demoi.get(membership.demos_id).await? {
                Some(d) => d,
                None => continue,
            };
            let threshold = feed_threshold(self.memberships.voter_count(demos.id).await?);
            for post in self.posts.list(demos.id).await? {
                if post.removed || post.pending_review {
                    continue;
                }
                let score = self.post_votes.score(post.id).await?;
                if score >= threshold {
                    items.push(FeedItem {
                        post,
                        score,
                        community_slug: demos.slug.clone(),
                    });
                }
            }
        }
        items.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(b.post.created_at.0.cmp(&a.post.created_at.0))
        });
        Ok(items)
    }

    /// The recommendation read use case.
    pub fn recommend_feed(&self) -> RecommendFeed {
        RecommendFeed::new(
            self.post_votes.clone(),
            self.recommender.clone(),
            self.posts.clone(),
            self.demoi.clone(),
        )
    }

    /// The recommendation model-refresh use case.
    pub fn refresh_recommendations(&self) -> RefreshRecommendations {
        RefreshRecommendations::new(self.post_votes.clone(), self.recommender.clone())
    }
}
