//! Whether an upload's bytes really are the type it claims to be.

use crate::{extension_for, sniff_content_type};

/// Whether `bytes` really are media of the declared `content_type`.
///
/// An upload arrives with a type the client chose, and nothing stops that client
/// from labelling an HTML document `image/png`. If we stored it under that label
/// it would later be served from our own origin with an image `Content-Type` —
/// and any browser (or intermediary) that sniffed past the header could treat it
/// as an active document. So the declared type is only ever a *hint*: it must
/// agree with what the bytes' magic number says they are.
///
/// The comparison is made through [`extension_for`](crate::extension_for) rather
/// than on the type strings, so the two sides are compared in canonical form and
/// a difference in spelling or parameters can't read as a mismatch.
///
/// Returns `false` — never an error — when the declared type is unsupported or
/// the bytes are unrecognisable, so callers treat "can't tell" exactly like
/// "doesn't match". This is a *cheap header check*, not proof of a well-formed
/// file; the sanitizer still decodes the media afterwards.
pub fn upload_matches_bytes(content_type: &str, bytes: &[u8]) -> bool {
    let Some(declared) = extension_for(content_type) else {
        return false;
    };
    let Some(sniffed) = sniff_content_type(bytes) else {
        return false;
    };
    extension_for(sniffed) == Some(declared)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];

    #[test]
    fn accepts_bytes_matching_their_declared_type() {
        assert!(upload_matches_bytes("image/png", PNG));
        assert!(upload_matches_bytes("image/jpeg", JPEG));
    }

    #[test]
    fn accepts_despite_parameters_and_casing() {
        assert!(upload_matches_bytes("IMAGE/PNG; charset=utf-8", PNG));
    }

    /// The case this function exists for: a document dressed as an image.
    #[test]
    fn rejects_a_document_declared_as_an_image() {
        let html = b"<!doctype html><script>alert(1)</script>";
        assert!(!upload_matches_bytes("image/png", html));
    }

    #[test]
    fn rejects_one_supported_type_declared_as_another() {
        assert!(!upload_matches_bytes("image/jpeg", PNG));
        assert!(!upload_matches_bytes("video/mp4", PNG));
    }

    #[test]
    fn rejects_unsupported_declared_type_even_with_valid_bytes() {
        assert!(!upload_matches_bytes("image/svg+xml", PNG));
        assert!(!upload_matches_bytes("application/zip", PNG));
    }

    #[test]
    fn rejects_unrecognisable_bytes() {
        assert!(!upload_matches_bytes("image/png", b""));
        assert!(!upload_matches_bytes("image/png", b"not media at all"));
    }
}
