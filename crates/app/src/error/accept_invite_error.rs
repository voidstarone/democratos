//! The error type of the invite-accept use-cases
//! ([`validate_invite_token`](crate::Services::validate_invite_token) and
//! [`mark_invite_accepted`](crate::Services::mark_invite_accepted)).

use thiserror::Error;

use crate::StoreError;

/// Why redeeming an invite link failed. `InvalidToken` is deliberately opaque —
/// an unknown token, an expired one, and an already-consumed one all look
/// identical, so a probe learns nothing.
#[derive(Debug, Error)]
pub enum AcceptInviteError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("this invite link is invalid or has expired")]
    InvalidToken,
}
