//! The error type of [`Services::open_proposal`](crate::Services::open_proposal).

use thiserror::Error;

use crate::StoreError;

/// Why opening a proposal was refused.
#[derive(Debug, Error)]
pub enum OpenProposalError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("user is not a voter of this demos")]
    NotAVoter,

    #[error("this member is under an active sanction")]
    Sanctioned,

    #[error("constitutional amendments are disabled while the demos is in its Seed phase")]
    ConstitutionalForbiddenInSeed,

    #[error("an open proposal with the same intent already exists for this demos")]
    DuplicateOpenProposal,
}
