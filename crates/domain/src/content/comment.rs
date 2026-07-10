//! A single forum comment.

use serde::{Deserialize, Serialize};

use crate::{CommentId, PostId, UserId};
use crate::time::Timestamp;

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Comment {
    pub id: CommentId,
    pub post_id: PostId,
    pub author: UserId,
    /// `None` for a top-level comment; otherwise the comment it replies to.
    pub parent: Option<CommentId>,
    pub body: String,
    pub created_at: Timestamp,
    pub removed: bool,
}

impl Comment {
    pub fn new(
        id: CommentId,
        post_id: PostId,
        author: UserId,
        parent: Option<CommentId>,
        body: impl Into<String>,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            post_id,
            author,
            parent,
            body: body.into(),
            created_at,
            removed: false,
        }
    }
}
