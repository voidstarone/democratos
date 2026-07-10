//! One entry in a member's personalized home feed.

use domain::Post;

/// One entry in a member's personalized home feed: a post that has cleared its
/// community's [`feed_threshold`](domain::feed_threshold), with its net score and
/// community slug.
#[derive(Clone, Debug)]
pub struct FeedItem {
    pub post: Post,
    pub score: i64,
    pub community_slug: String,
}
