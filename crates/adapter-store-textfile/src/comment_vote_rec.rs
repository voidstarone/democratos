//! A persisted comment up/down vote.

use serde::{Deserialize, Serialize};

use domain::{CommentId, UserId};

#[derive(Serialize, Deserialize)]
pub(crate) struct CommentVoteRec {
    pub(crate) comment: CommentId,
    pub(crate) user: UserId,
    pub(crate) up: bool,
}
