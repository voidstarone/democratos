//! An in-memory proposal vote.

use domain::{ProposalId, UserId};

pub(crate) struct VoteRec {
    pub(crate) proposal: ProposalId,
    pub(crate) voter: UserId,
    pub(crate) aye: bool,
    pub(crate) weight: u64,
}
