//! The error type of [`Services::authenticate`](crate::Services::authenticate).

use thiserror::Error;

use crate::StoreError;

/// Why a login failed. [`InvalidCredentials`](AuthenticateError::InvalidCredentials)
/// is deliberately undifferentiated so the response never reveals which accounts
/// exist.
#[derive(Debug, Error)]
pub enum AuthenticateError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("invalid email or password")]
    InvalidCredentials,
}
