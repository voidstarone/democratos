//! What a report points at.

use serde::{Deserialize, Serialize};

use crate::{CommentId, PostId, UserId};

/// What a report points at.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ReportTarget {
    Post(PostId),
    Comment(CommentId),
    User(UserId),
}
