//! The net score a post must reach to surface in a member's home feed.

/// The net score (upvotes − downvotes) a post must reach to surface in a
/// member's home feed. It **scales with the community**: roughly 10% of the
/// community's voters, with a floor of 1 — so a post needs broader support to
/// surface in a large demos than in a tiny one. One tunable place.
pub fn feed_threshold(community_voters: u64) -> i64 {
    let tenth = (community_voters + 9) / 10; // ceil(voters / 10)
    tenth.max(1) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_threshold_scales_with_community() {
        assert_eq!(feed_threshold(0), 1); // floor of 1 even with no voters
        assert_eq!(feed_threshold(1), 1);
        assert_eq!(feed_threshold(10), 1); // ceil(10/10)
        assert_eq!(feed_threshold(11), 2); // ceil(11/10)
        assert_eq!(feed_threshold(50), 5);
        assert_eq!(feed_threshold(100), 10);
    }
}
