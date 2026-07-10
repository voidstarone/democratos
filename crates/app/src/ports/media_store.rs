//! Byte storage for uploaded media.

use async_trait::async_trait;

use crate::{MediaError, Result};

/// Byte storage for uploaded media, kept behind a port so the backing store
/// (local disk now, a CDN/object-store later) is a one-line swap in the
/// composition root. The domain only ever stores the **URL** `put` returns,
/// never bytes.
#[async_trait]
pub trait MediaStore: Send + Sync {
    /// Persist `bytes` of the given MIME type; return a public URL to embed.
    async fn put(&self, content_type: &str, bytes: Vec<u8>) -> Result<String, MediaError>;
    /// Fetch stored bytes by key (the trailing segment of a local URL). A CDN
    /// adapter returns `Ok(None)` because its media is served by the CDN itself.
    async fn get(&self, key: &str) -> Result<Option<(String, Vec<u8>)>, MediaError>;
}
