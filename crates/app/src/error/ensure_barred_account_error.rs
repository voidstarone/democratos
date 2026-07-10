//! The error type of [`Services::ensure_barred_account`](crate::Services::ensure_barred_account).

use thiserror::Error;

use crate::StoreError;

/// Why provisioning a franchise-barred puppet account failed.
#[derive(Debug, Error)]
pub enum EnsureBarredAccountError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("{0}")]
    Rejected(String),
}
