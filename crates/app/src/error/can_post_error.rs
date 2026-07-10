//! The error type of [`Services::can_post`](crate::Services::can_post) and its
//! enforcing sibling `Services::require_can_post`.

use thiserror::Error;

use crate::StoreError;

/// Why posting was refused. [`Sanctioned`](CanPostError::Sanctioned) is its own
/// distinct error (blocks posting under any policy); [`Rejected`](CanPostError::Rejected)
/// carries the policy-specific message the composer can show.
#[derive(Debug, Error)]
pub enum CanPostError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("this member is under an active sanction")]
    Sanctioned,

    #[error("{0}")]
    Rejected(String),
}
