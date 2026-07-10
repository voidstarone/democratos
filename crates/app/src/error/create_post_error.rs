//! The error type of [`Services::create_post`](crate::Services::create_post).

use thiserror::Error;

use crate::error::can_post_error::CanPostError;
use crate::StoreError;

/// Why creating a post failed. The posting-policy gate
/// ([`require_can_post`](crate::Services::can_post)) folds in via
/// [`CanPost`](CreatePostError::CanPost); everything else is a store failure.
#[derive(Debug, Error)]
pub enum CreatePostError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    CanPost(#[from] CanPostError),
}
