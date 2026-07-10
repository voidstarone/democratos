//! The error type of [`Services::open_trial`](crate::Services::open_trial).

use thiserror::Error;

use crate::StoreError;

/// Why empanelling a jury for a report was refused.
#[derive(Debug, Error)]
pub enum OpenTrialError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("user is not a voter of this demos")]
    NotAVoter,

    #[error("the report is not open")]
    ReportNotOpen,

    #[error("the demos has too few voters to seat a minority jury")]
    JuryTooSmall,
}
