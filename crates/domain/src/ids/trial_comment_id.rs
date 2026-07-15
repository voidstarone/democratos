//! Identifies a comment on a trial.

use serde::{Deserialize, Serialize};

/// Identifies a comment on a trial (the trial's public gallery discussion,
/// distinct from a post [`crate::CommentId`]).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TrialCommentId(pub u64);

impl std::fmt::Display for TrialCommentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
