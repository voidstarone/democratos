//! The clean bytes a [`MediaSanitizer`](crate::MediaSanitizer) produces.

/// The result of sanitizing an upload: the bytes that are actually safe to store
/// and serve, paired with the canonical content type they should be stored under.
///
/// A sanitizer may *change* both the bytes (re-encoding an image strips any
/// embedded metadata, defuses decompression/polyglot payloads, and normalises the
/// container) and, in principle, the content type (e.g. transcoding). Callers
/// must persist these, never the client's original upload, so that what a CDN
/// later serves is a byte-for-byte product of our own encoder.
#[derive(Debug, Clone)]
pub struct SanitizedMedia {
    /// The canonical MIME type to store and serve the bytes under.
    pub content_type: String,
    /// The sanitized bytes to persist.
    pub bytes: Vec<u8>,
}

impl SanitizedMedia {
    pub fn new(content_type: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            content_type: content_type.into(),
            bytes,
        }
    }
}
