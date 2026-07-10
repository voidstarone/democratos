//! A post vote as persisted on disk.

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct PostVoteRec {
    pub(crate) post: u64,
    pub(crate) user: u64,
    pub(crate) up: bool,
}
