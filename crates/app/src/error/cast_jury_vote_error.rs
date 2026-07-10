//! The error type of [`Services::cast_jury_vote`](crate::Services::cast_jury_vote)
//! and the [`GovernanceWrites::cast_jury_vote`](crate::GovernanceWrites::cast_jury_vote)
//! port method.

use thiserror::Error;

use crate::error::settle_trial_error::SettleTrialError;
use crate::error::verify_action_error::VerifyActionError;
use crate::StoreError;

/// Why casting a juror's ballot was refused. [`Rejected`](CastJuryVoteError::Rejected)
/// carries a human-readable refusal: a missing/invalid signature locally, or an
/// owner rejection returned over the federation gateway.
#[derive(Debug, Error)]
pub enum CastJuryVoteError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("this user is not a juror on the trial")]
    NotAJuror,

    #[error("this member is under an active sanction")]
    Sanctioned,

    #[error("the trial has already reached a verdict")]
    TrialClosed,

    #[error("{0}")]
    Rejected(String),
}

/// The signature guard folds into this use-case's own vocabulary.
impl From<VerifyActionError> for CastJuryVoteError {
    fn from(e: VerifyActionError) -> Self {
        match e {
            VerifyActionError::Store(s) => CastJuryVoteError::Store(s),
            VerifyActionError::Rejected(m) => CastJuryVoteError::Rejected(m),
        }
    }
}

/// Settling the trial after the ballot ([`settle_trial`](crate::Services::settle_trial))
/// folds into this use-case's own vocabulary.
impl From<SettleTrialError> for CastJuryVoteError {
    fn from(e: SettleTrialError) -> Self {
        match e {
            SettleTrialError::Store(s) => CastJuryVoteError::Store(s),
            SettleTrialError::Sanctioned => CastJuryVoteError::Sanctioned,
        }
    }
}
