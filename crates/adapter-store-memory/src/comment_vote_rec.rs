//! An in-memory comment up/down vote.

use domain::{CommentId, UserId};

pub(crate) struct CommentVoteRec {
    pub(crate) comment: CommentId,
    pub(crate) user: UserId,
    pub(crate) up: bool,
}
