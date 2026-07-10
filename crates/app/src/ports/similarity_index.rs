//! The precomputed item→item similarity model behind the recommender.

use async_trait::async_trait;

use domain::{PostId, Rating};

use crate::Result;

/// The precomputed item→item similarity model behind the recommender. Kept a
/// port so the in-memory exact index (`adapter-recommend-memory`) can later be
/// swapped for an approximate-nearest-neighbour or matrix-factorisation backend
/// without touching `domain`, the use-cases, or any delivery adapter — the same
/// swap discipline as storage and media.
///
/// The model is built off the request path: [`rebuild`](Self::rebuild) ingests a
/// full ratings snapshot, [`neighbours`](Self::neighbours) then serves O(1)
/// lookups. [`version`](Self::version) lets a caller skip a rebuild when the
/// underlying votes are unchanged.
#[async_trait]
pub trait SimilarityIndex: Send + Sync {
    /// Rebuild the model from a full ratings snapshot, tagging it `version`.
    async fn rebuild(&self, version: u64, ratings: Vec<Rating>) -> Result<()>;
    /// The version stamped by the last successful [`rebuild`](Self::rebuild),
    /// or 0 if never built.
    async fn version(&self) -> u64;
    /// The precomputed neighbours of `post`, strongest similarity first.
    async fn neighbours(&self, post: PostId) -> Result<Vec<(PostId, f32)>>;
}
