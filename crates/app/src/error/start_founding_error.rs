//! The error type of [`Services::start_founding`](crate::Services::start_founding).

use thiserror::Error;

use crate::error::found_demos_error::FoundDemosError;
use crate::StoreError;

/// Why opening a founding petition failed: a store failure (including a taken slug,
/// which arrives as [`Store`](StartFoundingError::Store)) or a
/// [`Rejected`](StartFoundingError::Rejected) refusal (empty name, or a
/// franchise-barred founder).
#[derive(Debug, Error)]
pub enum StartFoundingError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("{0}")]
    Rejected(String),
}

/// The franchise-bar guard ([`found_demos`](crate::Services::found_demos)'s error)
/// folds into this use-case's own vocabulary: a store failure stays a store
/// failure, a bar rejection stays a rejection.
impl From<FoundDemosError> for StartFoundingError {
    fn from(e: FoundDemosError) -> Self {
        match e {
            FoundDemosError::Store(s) => StartFoundingError::Store(s),
            FoundDemosError::Rejected(m) => StartFoundingError::Rejected(m),
        }
    }
}
