//! A jury ballot as persisted on disk.

use serde::Deserialize;

use crate::backfill::one::one;

#[derive(Deserialize)]
pub(crate) struct JuryBallotRec {
    pub(crate) trial: u64,
    pub(crate) juror: u64,
    pub(crate) guilty: bool,
    #[serde(default = "one")]
    pub(crate) weight: u64,
}
