//! The error type of [`Services::vote_post`](crate::Services::vote_post) and the
//! [`GovernanceWrites::vote_post`](crate::GovernanceWrites::vote_post) port method.

use thiserror::Error;

use crate::error::member_action_error::MemberActionError;
use crate::error::verify_action_error::VerifyActionError;
use crate::StoreError;

/// Why casting a post up/down vote was refused. [`Rejected`](VotePostError::Rejected)
/// carries a human-readable refusal: a missing/invalid signature locally, or an
/// owner rejection returned over the federation gateway.
#[derive(Debug, Error)]
pub enum VotePostError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("this member is under an active sanction")]
    Sanctioned,

    #[error("{0}")]
    Rejected(String),
}

/// The unsanctioned-member gate folds into this use-case's own vocabulary.
impl From<MemberActionError> for VotePostError {
    fn from(e: MemberActionError) -> Self {
        match e {
            MemberActionError::Store(s) => VotePostError::Store(s),
            MemberActionError::Sanctioned => VotePostError::Sanctioned,
        }
    }
}

/// The signature guard folds into this use-case's own vocabulary.
impl From<VerifyActionError> for VotePostError {
    fn from(e: VerifyActionError) -> Self {
        match e {
            VerifyActionError::Store(s) => VotePostError::Store(s),
            VerifyActionError::Rejected(m) => VotePostError::Rejected(m),
        }
    }
}
