//! The error type of [`Services::register_account`](crate::Services::register_account).

use thiserror::Error;

use crate::StoreError;

/// Why registering a real account failed: a store failure, a credential mismatch,
/// or a policy [`Rejected`](RegisterAccountError::Rejected) refusal (invalid email
/// or password, a taken handle/email).
#[derive(Debug, Error)]
pub enum RegisterAccountError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("invalid email or password")]
    InvalidCredentials,

    #[error("{0}")]
    Rejected(String),
}
