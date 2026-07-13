//! The error type of the sensitive-content review use-cases.

use thiserror::Error;

use crate::StoreError;

/// Why a sensitive-content review action was refused.
#[derive(Debug, Error)]
pub enum SensitiveReviewError {
    #[error(transparent)]
    Store(#[from] StoreError),

    /// The acting account has not opted in to reviewing sensitive content.
    #[error("this account is not a sensitive-content reviewer")]
    NotReviewer,

    /// The case has already reached quorum and resolved.
    #[error("this case has already been resolved")]
    AlreadyResolved,

    /// A human-readable refusal (e.g. a self-flag or an unknown target).
    #[error("{0}")]
    Rejected(String),
}
