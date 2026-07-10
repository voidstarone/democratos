//! A proposal vote as persisted on disk.

use serde::Deserialize;

use crate::backfill::one::one;

/// A proposal vote as persisted on disk.
#[derive(Deserialize)]
pub(crate) struct VoteRec {
    pub(crate) proposal: u64,
    pub(crate) voter: u64,
    pub(crate) aye: bool,
    #[serde(default = "one")]
    pub(crate) weight: u64,
}
