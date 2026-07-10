//! The error type of [`Services::close_proposal`](crate::Services::close_proposal).

use thiserror::Error;

use crate::StoreError;

/// Why closing (tallying) a proposal was refused.
#[derive(Debug, Error)]
pub enum CloseProposalError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("the proposal's voting window has not closed yet")]
    VotingWindowOpen,
}
