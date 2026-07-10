//! The error type of content actions gated on unsanctioned membership
//! (`Services::require_unsanctioned_member`, and the [`comment`](crate::Services::comment)
//! / [`vote_comment`](crate::Services::vote_comment) use-cases that rely on it).

use thiserror::Error;

use crate::StoreError;

/// Why a member action was refused: a store failure, or the acting member is under
/// an active [`Sanctioned`](MemberActionError::Sanctioned). A non-member is a
/// [`Store`](MemberActionError::Store) `NotFound`.
#[derive(Debug, Error)]
pub enum MemberActionError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("this member is under an active sanction")]
    Sanctioned,
}
