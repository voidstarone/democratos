//! A comment with its replies nested beneath it.

use serde::{Deserialize, Serialize};

use crate::content::comment::Comment;

/// A comment with its replies nested beneath it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CommentNode {
    pub comment: Comment,
    pub children: Vec<CommentNode>,
}
