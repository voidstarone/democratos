//! Use case: recommend posts to a user from the votes of similar users.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use domain::{
    blend, rank_and_diversify, tag_affinity, tag_profile, DemosId, Post, PostId, UserId,
};

use crate::recommend::cold_start_min_likes::COLD_START_MIN_LIKES;
use crate::recommend::recommend_limit::RECOMMEND_LIMIT;
use crate::recommend::recommend_per_community_cap::RECOMMEND_PER_COMMUNITY_CAP;
use crate::recommend::recommendation::Recommendation;
use crate::{DemosStore, PostStore, PostVoteStore, Result, SimilarityIndex};

/// Use case: recommend posts to a user from the up/down votes of *similar* users
/// (item-based collaborative filtering, see [`domain::recommend`]).
///
/// A **pure read** of the precomputed model behind
/// [`SimilarityIndex`](crate::SimilarityIndex) — it never rebuilds; keeping the
/// model fresh is [`RefreshRecommendations`](crate::RefreshRecommendations)'s
/// job. It aggregates the neighbours of everything the user upvoted, drops
/// anything they have already voted on, and diversifies across communities. A
/// user with too little history (under [`COLD_START_MIN_LIKES`] upvotes) falls
/// back to tag affinity.
pub struct RecommendFeed {
    post_votes: Arc<dyn PostVoteStore>,
    recommender: Arc<dyn SimilarityIndex>,
    posts: Arc<dyn PostStore>,
    demoi: Arc<dyn DemosStore>,
}

impl RecommendFeed {
    pub fn new(
        post_votes: Arc<dyn PostVoteStore>,
        recommender: Arc<dyn SimilarityIndex>,
        posts: Arc<dyn PostStore>,
        demoi: Arc<dyn DemosStore>,
    ) -> Self {
        Self {
            post_votes,
            recommender,
            posts,
            demoi,
        }
    }

    /// Produce up to `limit` recommendations for `user` (a `limit` of 0 uses
    /// [`RECOMMEND_LIMIT`]).
    pub async fn execute(&self, user: UserId, limit: usize) -> Result<Vec<Recommendation>> {
        let limit = if limit == 0 { RECOMMEND_LIMIT } else { limit };

        // Seed from this user's upvotes; never recommend what they've already
        // voted on (up *or* down).
        let exclude: HashSet<PostId> = self.post_votes.voted_by(user).await?.into_iter().collect();
        let liked = self.post_votes.liked_by(user).await?;

        // Collaborative filtering when there's enough history, else tags.
        let scored = if liked.len() >= COLD_START_MIN_LIKES {
            let mut neighbour_lists = Vec::with_capacity(liked.len());
            for post in &liked {
                neighbour_lists.push(self.recommender.neighbours(*post).await?);
            }
            blend(&neighbour_lists, &exclude)
        } else {
            self.tag_fallback(&liked, &exclude).await?
        };
        if scored.is_empty() {
            return Ok(Vec::new());
        }

        // Resolve candidate posts (dropping removed ones), diversify, rank.
        let slugs: HashMap<DemosId, String> = self
            .demoi
            .list()
            .await?
            .into_iter()
            .map(|d| (d.id, d.slug))
            .collect();
        let mut candidates: Vec<(PostId, DemosId, f32)> = Vec::new();
        let mut posts: HashMap<PostId, Post> = HashMap::new();
        for (post_id, affinity) in scored {
            if let Some(post) = self.posts.get(post_id).await? {
                if post.removed {
                    continue;
                }
                candidates.push((post_id, post.demos_id, affinity));
                posts.insert(post_id, post);
            }
        }

        let ranked = rank_and_diversify(candidates, limit, RECOMMEND_PER_COMMUNITY_CAP);
        Ok(ranked
            .into_iter()
            .filter_map(|(post_id, demos_id, affinity)| {
                posts.remove(&post_id).map(|post| Recommendation {
                    post,
                    affinity,
                    community_slug: slugs.get(&demos_id).cloned().unwrap_or_default(),
                })
            })
            .collect())
    }

    /// Cold-start fallback: score un-voted posts by how well their tags overlap
    /// the tags of the posts this user liked.
    async fn tag_fallback(
        &self,
        liked: &[PostId],
        exclude: &HashSet<PostId>,
    ) -> Result<HashMap<PostId, f32>> {
        let mut liked_tags: Vec<Vec<String>> = Vec::new();
        for post in liked {
            if let Some(p) = self.posts.get(*post).await? {
                liked_tags.push(p.tags);
            }
        }
        let profile = tag_profile(&liked_tags);
        if profile.is_empty() {
            return Ok(HashMap::new());
        }
        let mut scored = HashMap::new();
        for post in self.posts.list_all().await? {
            if post.removed || exclude.contains(&post.id) {
                continue;
            }
            let affinity = tag_affinity(&profile, &post.tags);
            if affinity > 0.0 {
                scored.insert(post.id, affinity);
            }
        }
        Ok(scored)
    }
}
