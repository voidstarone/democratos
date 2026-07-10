//! The error type of [`Services::sign_founding`](crate::Services::sign_founding).

use thiserror::Error;

use crate::error::found_demos_error::FoundDemosError;
use crate::StoreError;

/// Why signing off on a pending founding failed: a store failure, or a
/// [`Rejected`](SignFoundingError::Rejected) refusal (the founder signing their
/// own, or a franchise-barred signer). When quorum lands the founding runs
/// [`found_demos`](crate::Services::found_demos), whose errors fold in here.
#[derive(Debug, Error)]
pub enum SignFoundingError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("{0}")]
    Rejected(String),
}

/// Founding the demos on quorum ([`found_demos`](crate::Services::found_demos))
/// folds into this use-case's own vocabulary.
impl From<FoundDemosError> for SignFoundingError {
    fn from(e: FoundDemosError) -> Self {
        match e {
            FoundDemosError::Store(s) => SignFoundingError::Store(s),
            FoundDemosError::Rejected(m) => SignFoundingError::Rejected(m),
        }
    }
}
