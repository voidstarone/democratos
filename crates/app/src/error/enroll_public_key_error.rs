//! The error type of [`Services::enroll_public_key`](crate::Services::enroll_public_key).

use thiserror::Error;

use crate::StoreError;

/// Why enrolling an account's signing key failed: a store failure, or a
/// [`Rejected`](EnrollPublicKeyError::Rejected) refusal (malformed key, or a key
/// already enrolled).
#[derive(Debug, Error)]
pub enum EnrollPublicKeyError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("{0}")]
    Rejected(String),
}
