//! A persisted proposal vote.

use serde::{Deserialize, Serialize};

use domain::{ProposalId, UserId};

#[derive(Serialize, Deserialize)]
pub(crate) struct VoteRec {
    pub(crate) proposal: ProposalId,
    pub(crate) voter: UserId,
    pub(crate) aye: bool,
    #[serde(default = "default_ballot_weight")]
    pub(crate) weight: u64,
}

/// Ballots persisted before vote weighting existed count as one-person-one-vote.
pub(crate) fn default_ballot_weight() -> u64 {
    1
}
