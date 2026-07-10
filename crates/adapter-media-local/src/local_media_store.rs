//! Local-filesystem implementation of the [`MediaStore`] port.

use std::path::{Path, PathBuf};

use async_trait::async_trait;

use app::MediaStore;
use app::{MediaError, Result};

pub struct LocalMediaStore {
    dir: PathBuf,
}

impl LocalMediaStore {
    /// Open the media directory, creating it (and parents) if absent.
    pub fn new(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Resolve a key to a path, rejecting anything that could escape `dir`.
    /// Keys are always `"<hash>.<ext>"` — no separators or parent references.
    fn path_for(&self, key: &str) -> Option<PathBuf> {
        if key.is_empty() || key.contains('/') || key.contains('\\') || key.contains("..") {
            return None;
        }
        Some(self.dir.join(key))
    }
}

#[async_trait]
impl MediaStore for LocalMediaStore {
    async fn put(&self, content_type: &str, bytes: Vec<u8>) -> Result<String, MediaError> {
        let key = app::media_key(content_type, &bytes)
            .ok_or_else(|| MediaError::Store(format!("unsupported media type: {content_type}")))?;
        let path = self
            .path_for(&key)
            .ok_or_else(|| MediaError::Store("bad media key".into()))?;
        // Content-addressed: identical bytes already on disk need no rewrite.
        if !path.exists() {
            std::fs::write(&path, &bytes)
                .map_err(|e| MediaError::Store(format!("media write: {e}")))?;
        }
        Ok(format!("/media/{key}"))
    }

    async fn get(&self, key: &str) -> Result<Option<(String, Vec<u8>)>, MediaError> {
        let Some(path) = self.path_for(key) else {
            return Ok(None);
        };
        let ext = key.rsplit('.').next().unwrap_or("");
        let content_type = app::content_type_for(ext).unwrap_or("application/octet-stream");
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some((content_type.to_string(), bytes))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(MediaError::Store(format!("media read: {e}"))),
        }
    }
}
