//! The error type of [`Services::settle_trial`](crate::Services::settle_trial).

use thiserror::Error;

use crate::StoreError;

/// Why recomputing (and, when decisive, applying) a trial's verdict failed.
#[derive(Debug, Error)]
pub enum SettleTrialError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("this member is under an active sanction")]
    Sanctioned,
}
