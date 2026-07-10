//! One entry in a user's recommended feed.

use domain::Post;

/// One entry in a user's recommended feed: a post they have **not** voted on,
/// with the affinity score that surfaced it and its community slug. Affinity is
/// a predicted-interest score, not a vote tally.
#[derive(Clone, Debug)]
pub struct Recommendation {
    pub post: Post,
    pub affinity: f32,
    pub community_slug: String,
}
