//! The error type of [`Services::request_invite`](crate::Services::request_invite).

use thiserror::Error;

use crate::StoreError;

/// Why recording an invite request failed. Note the service is deliberately
/// idempotent and enumeration-safe: an email that already has a request or an
/// account is *not* an error (it returns `Ok`), so this only fires on a genuine
/// store failure or a malformed email.
#[derive(Debug, Error)]
pub enum RequestInviteError {
    #[error(transparent)]
    Store(#[from] StoreError),

    /// The submitted email failed validation.
    #[error("{0}")]
    Rejected(String),
}
