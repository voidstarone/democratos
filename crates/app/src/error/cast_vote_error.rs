//! The error type of [`Services::cast_vote`](crate::Services::cast_vote) and the
//! [`GovernanceWrites::cast_vote`](crate::GovernanceWrites::cast_vote) port method.

use thiserror::Error;

use crate::error::verify_action_error::VerifyActionError;
use crate::StoreError;

/// Why casting a governance ballot was refused. [`Rejected`](CastVoteError::Rejected)
/// carries a human-readable refusal: a missing/invalid signature locally, or an
/// owner rejection returned over the federation gateway.
#[derive(Debug, Error)]
pub enum CastVoteError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("user is not a voter of this demos")]
    NotAVoter,

    #[error("the proposal is not open for voting")]
    ProposalNotOpen,

    #[error("this member is under an active sanction")]
    Sanctioned,

    #[error("the proposal's voting window has already closed")]
    VotingWindowClosed,

    #[error("{0}")]
    Rejected(String),
}

/// The signature guard folds into this use-case's own vocabulary.
impl From<VerifyActionError> for CastVoteError {
    fn from(e: VerifyActionError) -> Self {
        match e {
            VerifyActionError::Store(s) => CastVoteError::Store(s),
            VerifyActionError::Rejected(m) => CastVoteError::Rejected(m),
        }
    }
}
