//! Use case: rebuild the similarity model when the votes have changed.

use std::sync::Arc;

use domain::Rating;

use crate::{PostVoteStore, Result, SimilarityIndex};

/// Use case: rebuild the similarity model if the votes have changed since it was
/// last built. Depends only on the vote signal and the model.
///
/// Kept **off the request path** — a long-running deployment drives it from a
/// background task on an interval; a one-shot CLI invocation calls it once before
/// reading. Cheap when already current (just a [`vote_count`] comparison against
/// the model version); only snapshots the full vote history when actually stale.
///
/// [`vote_count`]: crate::PostVoteStore::vote_count
pub struct RefreshRecommendations {
    post_votes: Arc<dyn PostVoteStore>,
    recommender: Arc<dyn SimilarityIndex>,
}

impl RefreshRecommendations {
    pub fn new(post_votes: Arc<dyn PostVoteStore>, recommender: Arc<dyn SimilarityIndex>) -> Self {
        Self {
            post_votes,
            recommender,
        }
    }

    /// Rebuild if stale; returns whether a rebuild happened.
    pub async fn execute(&self) -> Result<bool> {
        let version = self.post_votes.vote_count().await?;
        if self.recommender.version().await == version {
            return Ok(false);
        }
        let ratings: Vec<Rating> = self
            .post_votes
            .all_votes()
            .await?
            .into_iter()
            .map(|(post, voter, up)| Rating::from_vote(voter, post, up))
            .collect();
        self.recommender.rebuild(version, ratings).await?;
        Ok(true)
    }
}
