//! A persisted jury ballot.

use serde::{Deserialize, Serialize};

use domain::{TrialId, UserId};

#[derive(Serialize, Deserialize)]
pub(crate) struct JuryBallotRec {
    pub(crate) trial: TrialId,
    pub(crate) juror: UserId,
    pub(crate) guilty: bool,
    #[serde(default = "crate::vote_rec::default_ballot_weight")]
    pub(crate) weight: u64,
}
