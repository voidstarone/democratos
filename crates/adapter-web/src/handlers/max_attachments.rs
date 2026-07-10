//! The most media attachments one post may carry.

/// The most media attachments one post may carry. Bounds both the number of
/// stored blobs a single request can create and — together with the per-file cap
/// and streaming-then-persisting — the total bytes a request can push. Also feeds
/// the `/posts` body limit in [`crate::router`].
pub(crate) const MAX_ATTACHMENTS: usize = 8;
