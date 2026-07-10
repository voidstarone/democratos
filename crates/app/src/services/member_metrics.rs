//! A member's earned standing in one community.

/// A member's earned standing in one community, computed from real engagement
/// (not a hand-incremented counter). Its [`popularity`](Self::popularity) is the
/// canonical "contribution" that gates the franchise and posting.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MemberMetrics {
    /// Non-removed posts authored in this community.
    pub posts: u32,
    /// Non-removed comments authored in this community.
    pub comments: u32,
    /// Net upvotes (up − down) across those posts.
    pub net_post_upvotes: i64,
    /// Net upvotes across those comments.
    pub net_comment_upvotes: i64,
}

impl MemberMetrics {
    /// Popularity: net upvotes on the member's posts **and** comments here. This
    /// is the value cached as `Membership::contribution`.
    pub fn popularity(&self) -> i64 {
        self.net_post_upvotes + self.net_comment_upvotes
    }
}
