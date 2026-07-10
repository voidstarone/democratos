//! An [`SimilarityIndex`] that keeps the precomputed model in process memory.

use std::sync::RwLock;

use async_trait::async_trait;

use app::{Result, SimilarityIndex};
use domain::{ItemIndex, PostId, Rating};

struct State {
    version: u64,
    index: ItemIndex,
}

/// An [`SimilarityIndex`] that keeps the model in process memory.
pub struct MemoryRecommender {
    neighbours_per_post: usize,
    state: RwLock<State>,
}

impl MemoryRecommender {
    /// Build an empty recommender keeping `neighbours_per_post` neighbours per
    /// post. Version starts at 0 (never built), so the first request triggers a
    /// rebuild.
    pub fn new(neighbours_per_post: usize) -> Self {
        Self {
            neighbours_per_post,
            state: RwLock::new(State {
                version: 0,
                index: ItemIndex::default(),
            }),
        }
    }
}

impl Default for MemoryRecommender {
    fn default() -> Self {
        Self::new(crate::DEFAULT_NEIGHBOURS)
    }
}

#[async_trait]
impl SimilarityIndex for MemoryRecommender {
    async fn rebuild(&self, version: u64, ratings: Vec<Rating>) -> Result<()> {
        let index = ItemIndex::build(&ratings, self.neighbours_per_post);
        let mut state = self.state.write().expect("recommender lock poisoned");
        state.version = version;
        state.index = index;
        Ok(())
    }

    async fn version(&self) -> u64 {
        self.state
            .read()
            .expect("recommender lock poisoned")
            .version
    }

    async fn neighbours(&self, post: PostId) -> Result<Vec<(PostId, f32)>> {
        Ok(self
            .state
            .read()
            .expect("recommender lock poisoned")
            .index
            .neighbours(post)
            .to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::UserId;

    #[tokio::test]
    async fn rebuild_updates_version_and_serves_neighbours() {
        let rec = MemoryRecommender::default();
        assert_eq!(rec.version().await, 0);

        // Two users who both upvote posts 1 and 2; user 2 adds variance.
        let ratings = vec![
            Rating::from_vote(UserId(1), PostId(1), true),
            Rating::from_vote(UserId(1), PostId(2), true),
            Rating::from_vote(UserId(2), PostId(1), true),
            Rating::from_vote(UserId(2), PostId(2), true),
            Rating::from_vote(UserId(2), PostId(3), false),
        ];
        rec.rebuild(5, ratings).await.unwrap();

        assert_eq!(rec.version().await, 5);
        let n = rec.neighbours(PostId(1)).await.unwrap();
        assert!(n.iter().any(|(p, _)| *p == PostId(2)));
    }
}
