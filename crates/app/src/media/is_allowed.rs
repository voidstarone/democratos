//! Whether an uploaded content type is one we accept at all.

use crate::extension_for;

/// Whether `content_type` is a media type this server accepts.
///
/// The cheap first gate on the upload path: it rejects a type we have no business
/// storing before any bytes are examined. It is deliberately *only* a check of
/// the client's claim — passing it means the declared type is on the allowlist,
/// not that the bytes are really that type. That second, load-bearing question is
/// [`upload_matches_bytes`](crate::upload_matches_bytes)'s, and the sanitizer
/// re-derives the type again from the bytes it can actually decode. Both run
/// after this one; none of them may be skipped on the strength of it.
///
/// Defined in terms of [`extension_for`](crate::extension_for) so the allowlist
/// exists in exactly one place.
pub fn is_allowed(content_type: &str) -> bool {
    extension_for(content_type).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_allowlisted_types() {
        for ct in [
            "image/png",
            "image/jpeg",
            "image/gif",
            "image/webp",
            "video/mp4",
            "video/webm",
        ] {
            assert!(is_allowed(ct), "{ct} should be allowed");
        }
    }

    #[test]
    fn rejects_documents_and_archives() {
        assert!(!is_allowed("text/html"));
        assert!(!is_allowed("application/zip"));
        assert!(!is_allowed("application/pdf"));
        assert!(!is_allowed("image/svg+xml"));
    }

    #[test]
    fn tracks_extension_for_including_normalisation() {
        assert!(is_allowed("image/png; charset=utf-8"));
        assert!(!is_allowed(""));
    }
}
