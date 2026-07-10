//! Turns an untrusted upload into bytes that are safe to store and serve.

use async_trait::async_trait;

use crate::ports::sanitized_media::SanitizedMedia;
use crate::MediaError;

/// Turns an untrusted upload into bytes that are safe to store and serve — the
/// defence against *malicious media*, distinct from the illegal-content scan the
/// [`MediaSafetyScanner`](crate::MediaSafetyScanner) performs.
///
/// A real implementation decodes the upload (proving it is genuinely the media it
/// claims to be, not a polyglot), rejects decompression / pixel bombs by bounding
/// dimensions from the header *before* allocating pixels, and re-encodes images so
/// what we persist carries none of the original's metadata or trailing payload.
/// Kept behind a port so the heavy codec dependency lives in an adapter, never in
/// the application or domain, and so a node too small to decode media can wire a
/// cheaper implementation.
#[async_trait]
pub trait MediaSanitizer: Send + Sync {
    /// Validate and clean an upload declared as `declared_content_type`. Returns
    /// the bytes to actually store (and the type to store them under), or
    /// [`MediaError::Rejected`] if the bytes are not safe, decodable media of the
    /// declared kind. The declared type is advisory — the sanitizer trusts only
    /// what it can decode.
    async fn sanitize(
        &self,
        declared_content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<SanitizedMedia, MediaError>;
}
