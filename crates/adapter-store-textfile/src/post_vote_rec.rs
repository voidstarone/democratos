//! A persisted post up/down vote.

use serde::{Deserialize, Serialize};

use domain::{PostId, UserId};

#[derive(Serialize, Deserialize)]
pub(crate) struct PostVoteRec {
    pub(crate) post: PostId,
    pub(crate) user: UserId,
    pub(crate) up: bool,
}
