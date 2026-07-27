//! Derive a media type from the bytes themselves, never from the client's claim.

/// The media type `bytes` actually are, judged by their magic number, or `None`
/// if they are not recognisably one of the supported types.
///
/// The client's declared `Content-Type` is an assertion by an attacker-controlled
/// party; this is the server's own reading of the file. It is what makes
/// [`upload_matches_bytes`](crate::upload_matches_bytes) meaningful, and what
/// stops a document from being stored under an image type and later served back
/// as something a browser might sniff and execute.
///
/// Only the header is inspected, so this is cheap and allocation-free — it proves
/// the *framing* is right, not that the whole file is well-formed. Full
/// validation is the sanitizer's job: it decodes images outright and structurally
/// validates video containers.
pub fn sniff_content_type(bytes: &[u8]) -> Option<&'static str> {
    // PNG: the 8-byte signature, whose \r\n and ^Z bytes also detect transfers
    // that mangled line endings.
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }
    // JPEG: SOI marker followed by the start of any segment marker.
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    // GIF: both revisions of the header block.
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    // WebP: a RIFF container whose form type (bytes 8..12) is WEBP. The length
    // field between them is not trusted, so it is skipped rather than parsed.
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    // MP4 / ISO-BMFF: the first box is `ftyp` at offset 4. Its preceding 4 bytes
    // are that box's length, which varies, so the brand box is matched directly
    // instead of trusting the length.
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        return Some("video/mp4");
    }
    // WebM: the EBML magic is shared with Matroska, which is *not* on the
    // allowlist, so the magic alone is not enough. The DocType element naming the
    // profile sits in the EBML header at the very start of the file; requiring
    // the literal "webm" there separates the two without parsing EBML properly.
    // A Matroska file therefore sniffs as `None` and is refused, which is the
    // correct outcome for a type we do not accept.
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        let header = &bytes[..bytes.len().min(64)];
        if header.windows(4).any(|w| w == b"webm") {
            return Some("video/webm");
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_png() {
        let png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        assert_eq!(sniff_content_type(png), Some("image/png"));
    }

    #[test]
    fn detects_jpeg() {
        assert_eq!(
            sniff_content_type(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]),
            Some("image/jpeg")
        );
    }

    #[test]
    fn detects_both_gif_revisions() {
        assert_eq!(sniff_content_type(b"GIF87a...."), Some("image/gif"));
        assert_eq!(sniff_content_type(b"GIF89a...."), Some("image/gif"));
    }

    #[test]
    fn detects_webp_only_with_the_riff_form_type() {
        assert_eq!(
            sniff_content_type(b"RIFF\x24\x00\x00\x00WEBPVP8 "),
            Some("image/webp")
        );
        // A RIFF container that is not WebP (e.g. a WAV) is not media we accept.
        assert_eq!(sniff_content_type(b"RIFF\x24\x00\x00\x00WAVEfmt "), None);
    }

    #[test]
    fn detects_mp4_by_the_ftyp_box() {
        let mp4 = b"\0\0\0\x18ftypmp42 rest of file";
        assert_eq!(sniff_content_type(mp4), Some("video/mp4"));
    }

    #[test]
    fn detects_webm_by_doctype() {
        let webm = b"\x1a\x45\xdf\xa3\x01\x00\x00\x00\x42\x82\x84webm rest";
        assert_eq!(sniff_content_type(webm), Some("video/webm"));
    }

    /// Matroska shares WebM's EBML magic but is not on the allowlist, so it must
    /// not be admitted by the magic number alone.
    #[test]
    fn does_not_mistake_matroska_for_webm() {
        let mkv = b"\x1a\x45\xdf\xa3\x01\x00\x00\x00\x42\x82\x88matroska rest";
        assert_eq!(sniff_content_type(mkv), None);
    }

    #[test]
    fn rejects_documents_and_truncated_input() {
        assert_eq!(sniff_content_type(b"<!doctype html><script>"), None);
        assert_eq!(sniff_content_type(b"PK\x03\x04"), None);
        assert_eq!(sniff_content_type(b""), None);
        assert_eq!(sniff_content_type(b"RIFF"), None);
    }
}
