//! One observed rating signal.

use crate::{PostId, UserId};

/// One observed signal: how a user rated a post. `+1.0` for an upvote, `-1.0`
/// for a downvote — the same one-vote-per-member signal the home feed uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rating {
    pub user: UserId,
    pub post: PostId,
    pub value: f32,
}

impl Rating {
    /// Build a rating from a boolean up/down vote (`true` = up).
    pub fn from_vote(user: UserId, post: PostId, up: bool) -> Self {
        Self {
            user,
            post,
            value: if up { 1.0 } else { -1.0 },
        }
    }
}
