//! A [`MediaStore`] backed by a shared S3/MinIO bucket.

use async_trait::async_trait;
use s3::creds::Credentials;
use s3::{Bucket, Region};

use app::MediaStore;
use app::{MediaError, Result};

use crate::S3Config;

/// A [`MediaStore`] backed by a shared S3/MinIO bucket.
pub struct S3MediaStore {
    bucket: Box<Bucket>,
    public_base: Option<String>,
}

impl S3MediaStore {
    /// Build a store from `config`. Does not create the bucket — see
    /// [`ensure_bucket`](Self::ensure_bucket).
    pub fn new(config: S3Config) -> Result<Self, MediaError> {
        let region = Region::Custom {
            region: config.region,
            endpoint: config.endpoint,
        };
        let creds = Credentials::new(
            Some(&config.access_key),
            Some(&config.secret_key),
            None,
            None,
            None,
        )
        .map_err(|e| MediaError::Store(format!("s3 credentials: {e}")))?;

        let mut bucket = Bucket::new(&config.bucket, region, creds)
            .map_err(|e| MediaError::Store(format!("s3 bucket: {e}")))?;
        if config.uses_path_style {
            bucket = bucket.with_path_style();
        }
        Ok(Self {
            bucket,
            public_base: config
                .public_base
                .map(|b| b.trim_end_matches('/').to_string()),
        })
    }

    /// Create the bucket if it does not already exist. Safe to call at boot;
    /// idempotent (an already-owned bucket is treated as success).
    pub async fn ensure_bucket(&self) -> Result<(), MediaError> {
        if self.bucket.exists().await.unwrap_or(false) {
            return Ok(());
        }
        let region = self.bucket.region();
        let creds = self
            .bucket
            .credentials()
            .await
            .map_err(|e| MediaError::Store(format!("s3 credentials: {e}")))?;
        let config = s3::BucketConfiguration::private();
        match Bucket::create_with_path_style(self.bucket.name().as_str(), region, creds, config)
            .await
        {
            Ok(_) => Ok(()),
            // A concurrent creator or a pre-existing bucket we own is fine.
            Err(_) if self.bucket.exists().await.unwrap_or(false) => Ok(()),
            Err(e) => Err(MediaError::Store(format!("s3 create bucket: {e}"))),
        }
    }

    /// Reject keys that could address something other than a single top-level
    /// object (defence in depth; keys we mint never contain these).
    fn is_safe_key(key: &str) -> bool {
        !key.is_empty() && !key.contains('/') && !key.contains('\\') && !key.contains("..")
    }
}

#[async_trait]
impl MediaStore for S3MediaStore {
    async fn put(&self, content_type: &str, bytes: Vec<u8>) -> Result<String, MediaError> {
        let key = app::media_key(content_type, &bytes)
            .ok_or_else(|| MediaError::Store(format!("unsupported media type: {content_type}")))?;

        // Store the canonical MIME for the resolved extension, never the raw
        // client-supplied `content_type` (which may carry trailing `;params`).
        // The type is pinned to the allowlist, so it can never become executable,
        // and the served object's Content-Type matches the proxied `get` path's.
        let ext = key.rsplit('.').next().unwrap_or("");
        let store_type = app::content_type_for(ext).unwrap_or("application/octet-stream");

        // Content-addressed: an object with these exact bytes is already there.
        let present = matches!(self.bucket.head_object(&key).await, Ok((_, code)) if code == 200);
        if !present {
            let resp = self
                .bucket
                .put_object_with_content_type(&key, &bytes, store_type)
                .await
                .map_err(|e| MediaError::Store(format!("s3 put: {e}")))?;
            let code = resp.status_code();
            if !(200..300).contains(&code) {
                return Err(MediaError::Store(format!("s3 put returned HTTP {code}")));
            }
        }

        Ok(match &self.public_base {
            Some(base) => format!("{base}/{key}"),
            None => format!("/media/{key}"),
        })
    }

    async fn get(&self, key: &str) -> Result<Option<(String, Vec<u8>)>, MediaError> {
        // In direct/CDN mode the bytes are served elsewhere, never proxied.
        if self.public_base.is_some() {
            return Ok(None);
        }
        if !Self::is_safe_key(key) {
            return Ok(None);
        }
        let ext = key.rsplit('.').next().unwrap_or("");
        let content_type = app::content_type_for(ext).unwrap_or("application/octet-stream");

        match self.bucket.get_object(key).await {
            Ok(resp) => {
                let code = resp.status_code();
                if code == 404 {
                    return Ok(None);
                }
                if !(200..300).contains(&code) {
                    return Err(MediaError::Store(format!("s3 get returned HTTP {code}")));
                }
                Ok(Some((content_type.to_string(), resp.to_vec())))
            }
            // rust-s3 surfaces a 404 as an error on some backends; treat "not
            // found" as absence, anything else as a real failure.
            Err(s3::error::S3Error::HttpFailWithBody(404, _)) => Ok(None),
            Err(e) => Err(MediaError::Store(format!("s3 get: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::S3MediaStore;

    #[test]
    fn rejects_keys_that_escape_a_single_object() {
        assert!(!S3MediaStore::is_safe_key(""));
        assert!(!S3MediaStore::is_safe_key("a/b.png"));
        assert!(!S3MediaStore::is_safe_key("..%2f.png"));
        assert!(!S3MediaStore::is_safe_key("../secret"));
        assert!(S3MediaStore::is_safe_key("deadbeef00000000.png"));
    }
}
