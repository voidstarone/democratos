//! The error type of the shared signature-verification guard
//! (`Services::verify_user_action`).

use thiserror::Error;

use crate::StoreError;

/// Why verifying that a user authorised an action failed: a store failure (loading
/// the account, or a malformed enrolled key surfaced as
/// [`Store`](VerifyActionError::Store)), or a [`Rejected`](VerifyActionError::Rejected)
/// refusal (a required signature missing or not verifying). Folds into each
/// governance write's own error vocabulary.
#[derive(Debug, Error)]
pub enum VerifyActionError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("{0}")]
    Rejected(String),
}
