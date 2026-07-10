//! The error type of [`Services::set_feed_paging`](crate::Services::set_feed_paging).

use thiserror::Error;

use crate::StoreError;

/// Why persisting a member's feed-paging preference failed.
#[derive(Debug, Error)]
pub enum SetFeedPagingError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("{0}")]
    Rejected(String),
}
