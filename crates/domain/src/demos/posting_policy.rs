//! Who is allowed to create posts in a community.

use serde::{Deserialize, Serialize};

/// Who is allowed to create posts in a community. From most open to most
/// restrictive; a community sets it by governance vote.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum PostingPolicy {
    /// Any signed-in user, member or not.
    Open,
    /// Joined members in good standing (the default).
    Members,
    /// Enfranchised voters only.
    Voters,
    /// Members whose popularity (net upvotes on their posts + comments here)
    /// meets a threshold — the "earn your way in" setting.
    MinContribution(i64),
}

impl Default for PostingPolicy {
    fn default() -> Self {
        PostingPolicy::Members
    }
}
