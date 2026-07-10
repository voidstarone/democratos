//! An in-memory post up/down vote.

use domain::{PostId, UserId};

pub(crate) struct PostVoteRec {
    pub(crate) post: PostId,
    pub(crate) user: UserId,
    pub(crate) up: bool,
}
