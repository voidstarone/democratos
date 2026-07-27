//! The canonical MIME type for a supported file extension.

/// The canonical MIME type to serve a file with extension `ext` under, or `None`
/// if the extension is not one we support.
///
/// This is the read-path counterpart to
/// [`extension_for`](crate::extension_for). Stored media is keyed as
/// `"<hash>.<ext>"`, so serving it back means turning that extension into a
/// `Content-Type` — and it must be *our* canonical type, never anything derived
/// from what the uploader declared, so that the header a browser finally sees is
/// pinned to this closed list and cannot be steered toward an executable type.
///
/// It accepts `jpeg` as well as `jpg` because it is also used to vet the
/// extension of a *linked* media URL, and both spellings are common in the wild;
/// `extension_for` only ever mints `jpg`, so keys stay canonical either way.
pub fn content_type_for(ext: &str) -> Option<&'static str> {
    // Tolerate a leading dot and any casing — this is fed by filenames, URL
    // paths, and storage keys, which are not uniform about either.
    let e = ext.trim().trim_start_matches('.').to_ascii_lowercase();

    match e.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "mp4" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension_for;

    #[test]
    fn maps_every_supported_extension() {
        assert_eq!(content_type_for("png"), Some("image/png"));
        assert_eq!(content_type_for("jpg"), Some("image/jpeg"));
        assert_eq!(content_type_for("gif"), Some("image/gif"));
        assert_eq!(content_type_for("webp"), Some("image/webp"));
        assert_eq!(content_type_for("mp4"), Some("video/mp4"));
        assert_eq!(content_type_for("webm"), Some("video/webm"));
    }

    #[test]
    fn accepts_both_jpeg_spellings() {
        assert_eq!(content_type_for("jpeg"), Some("image/jpeg"));
        assert_eq!(content_type_for("jpg"), Some("image/jpeg"));
    }

    #[test]
    fn tolerates_leading_dot_and_casing() {
        assert_eq!(content_type_for(".PNG"), Some("image/png"));
        assert_eq!(content_type_for("WebM"), Some("video/webm"));
    }

    #[test]
    fn rejects_unsupported_extensions() {
        assert_eq!(content_type_for("zip"), None);
        assert_eq!(content_type_for("svg"), None);
        assert_eq!(content_type_for(""), None);
    }

    /// Every extension the allowlist mints must map back to the type it came
    /// from, or a stored key would be served under the wrong `Content-Type`.
    #[test]
    fn round_trips_with_extension_for() {
        for ct in [
            "image/png",
            "image/jpeg",
            "image/gif",
            "image/webp",
            "video/mp4",
            "video/webm",
        ] {
            let ext = extension_for(ct).expect("allowlisted type has an extension");
            assert_eq!(content_type_for(ext), Some(ct), "round-trip failed for {ct}");
        }
    }
}
