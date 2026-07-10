//! An in-memory jury ballot.

use domain::{TrialId, UserId};

pub(crate) struct JuryBallotRec {
    pub(crate) trial: TrialId,
    pub(crate) juror: UserId,
    pub(crate) guilty: bool,
    pub(crate) weight: u64,
}
