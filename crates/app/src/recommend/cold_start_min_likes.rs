//! The cold-start threshold for collaborative filtering.

/// Below this many upvotes a user has too little history for collaborative
/// filtering; recommendations fall back to tag affinity instead.
pub const COLD_START_MIN_LIKES: usize = 3;
