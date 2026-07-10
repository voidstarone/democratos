//! The error vocabulary of the [`MediaStore`](crate::MediaStore) port.

use thiserror::Error;

/// Failures storing or serving uploaded media. [`Store`](MediaError::Store) is an
/// infrastructure failure (its message is prefixed for logs); [`Rejected`](MediaError::Rejected)
/// is a human-readable refusal (unsupported type, uploads disabled) that renders
/// verbatim.
#[derive(Debug, Error)]
pub enum MediaError {
    #[error("storage failure: {0}")]
    Store(String),

    #[error("{0}")]
    Rejected(String),
}
