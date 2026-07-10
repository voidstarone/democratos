//! A [`MediaStore`] that hosts nothing — for a node that opts out of media hosting.

use async_trait::async_trait;

use app::MediaStore;
use app::{MediaError, Result};

/// A [`MediaStore`] that hosts nothing — for a node that opts OUT of media
/// hosting (e.g. a small, slow, storage-light box). Uploads are cleanly refused
/// and no bytes are ever stored or served locally, so the box carries no media
/// weight. Text/governance keep working; posting a file returns a clear error.
/// A media-light node that should still *show* media points at a shared object
/// store instead (`--media s3` at another box's bucket).
pub struct DisabledMediaStore;

#[async_trait]
impl MediaStore for DisabledMediaStore {
    async fn put(&self, _content_type: &str, _bytes: Vec<u8>) -> Result<String, MediaError> {
        Err(MediaError::Rejected(
            "media hosting is disabled on this node".into(),
        ))
    }

    async fn get(&self, _key: &str) -> Result<Option<(String, Vec<u8>)>, MediaError> {
        // Nothing is stored here, so every key is a miss → the web layer 404s.
        Ok(None)
    }
}
