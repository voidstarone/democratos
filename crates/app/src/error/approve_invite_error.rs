//! The error type of [`Services::approve_invite`](crate::Services::approve_invite).

use thiserror::Error;

use crate::{NotifyError, StoreError};

/// Why approving an invite request failed.
///
/// Ordering of effects matters: the request is only marked approved *after* the
/// email is accepted for delivery, so a [`Notify`](ApproveInviteError::Notify)
/// failure leaves the request pending and safely retryable — never approved-but-
/// silent.
#[derive(Debug, Error)]
pub enum ApproveInviteError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    Notify(#[from] NotifyError),

    /// No pending request with that id — unknown, or already decided.
    #[error("no pending request with that id")]
    NotPending,
}
