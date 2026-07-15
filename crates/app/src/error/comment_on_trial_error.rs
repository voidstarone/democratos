//! The error type of
//! [`Services::comment_on_trial`](crate::Services::comment_on_trial).

use thiserror::Error;

use crate::StoreError;

/// Why posting a comment on a trial was refused.
#[derive(Debug, Error)]
pub enum CommentOnTrialError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("only an enfranchised voter of this demos may comment on its trials")]
    NotAVoter,

    #[error("a comment cannot be empty")]
    Empty,
}
