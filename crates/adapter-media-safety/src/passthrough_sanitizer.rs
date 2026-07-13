//! A sanitizer that validates the type but does not re-encode.

use async_trait::async_trait;

use app::{MediaError, MediaSanitizer, SanitizedMedia};

/// A [`MediaSanitizer`] that verifies an upload's bytes match its declared type
/// (via the magic-number sniff) and canonicalises the content type, but does
/// **not** decode or re-encode. It still refuses type/content mismatches and
/// unsupported types, but cannot strip metadata, defuse a decompression bomb, or
/// remove a trailing polyglot payload.
///
/// Offered only for nodes too CPU-constrained to re-encode (a very small box).
/// [`ImageReencodeSanitizer`](crate::ImageReencodeSanitizer) is the default and
/// the one to prefer wherever the hardware allows.
#[derive(Default)]
pub struct PassthroughSanitizer;

#[async_trait]
impl MediaSanitizer for PassthroughSanitizer {
    async fn sanitize(
        &self,
        declared_content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<SanitizedMedia, MediaError> {
        if !app::is_allowed(declared_content_type) {
            return Err(MediaError::Rejected(format!(
                "unsupported upload type: {declared_content_type}"
            )));
        }
        if !app::upload_matches_bytes(declared_content_type, &bytes) {
            return Err(MediaError::Rejected(
                "that file's contents don't match its type".to_string(),
            ));
        }
        // Canonicalise: store under the type derived from the bytes' extension.
        let ext = app::extension_for(declared_content_type)
            .ok_or_else(|| MediaError::Rejected("unsupported upload type".to_string()))?;
        let canonical = app::content_type_for(ext)
            .ok_or_else(|| MediaError::Rejected("unsupported upload type".to_string()))?;
        Ok(SanitizedMedia::new(canonical, bytes))
    }
}
