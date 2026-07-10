//! Per-community cap on a cross-community discovery feed.

/// At most this many recommendations may come from any one community, so a
/// single large demos cannot dominate a cross-community discovery feed.
pub const RECOMMEND_PER_COMMUNITY_CAP: usize = 5;
