//! Build the media backend chosen on the CLI.

use std::sync::Arc;

use anyhow::Result;

use adapter_media_local::{DisabledMediaStore, LocalMediaStore};
use adapter_media_s3::{S3Config, S3MediaStore};

use crate::media_kind::MediaKind;

/// THE swap point. Choosing a storage backend is one `match`; the returned
/// `Services` is identical regardless of which arm runs. Note `media`: the
/// memory arm keeps bytes in RAM, the file arm writes them to disk via
/// `LocalMediaStore` — and a CDN/object-store would be one more `Arc::new` here,
/// with no change anywhere else.
/// Build the media backend chosen on the CLI. Returned as a trait object so the
/// rest of `Services` is identical whichever backend runs — the object-store
/// swap the federation needs is entirely contained here.
pub(crate) async fn build_media_store(
    kind: MediaKind,
    media_dir: &str,
    s3: Option<S3Config>,
) -> Result<Arc<dyn app::MediaStore>> {
    match kind {
        MediaKind::Local => Ok(Arc::new(LocalMediaStore::new(media_dir)?)),
        MediaKind::S3 => {
            let config = s3.ok_or_else(|| {
                anyhow::anyhow!(
                    "--media s3 requires --s3-endpoint and credentials \
                     (--s3-access-key / --s3-secret-key or AWS_* env)"
                )
            })?;
            let store = S3MediaStore::new(config)?;
            // Create the bucket on first boot; idempotent if it already exists.
            store.ensure_bucket().await?;
            Ok(Arc::new(store))
        }
        MediaKind::None => Ok(Arc::new(DisabledMediaStore)),
    }
}
