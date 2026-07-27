//! The content-addressed storage key for a piece of media.

use sha2::{Digest, Sha256};

use crate::extension_for;

/// The storage key for media of `content_type` with these `bytes`:
/// `"<sha256-hex>.<ext>"`. `None` if the type is not on the allowlist.
///
/// The key is **content-addressed**: it is derived entirely from the bytes, so
/// identical uploads collapse onto one object (every backend can skip the write
/// when the key already exists) and a key can never be steered by anything the
/// uploader supplies — no filename, no path, no client-chosen id ever reaches
/// storage. That property is what lets stores treat the key as a safe path
/// segment, and what makes the immutable one-year `Cache-Control` on the serve
/// path sound: a given key's bytes can never change.
///
/// The extension comes from [`extension_for`](crate::extension_for), so it is one
/// of a closed set of known-safe values and round-trips back to the canonical
/// MIME type via [`content_type_for`](crate::content_type_for) when the object is
/// served. Callers pass the type the *sanitizer* settled on, not the client's
/// declared one.
///
/// SHA-256 rather than a fast non-cryptographic hash because a collision here
/// would mean one upload silently overwriting or impersonating another.
pub fn media_key(content_type: &str, bytes: &[u8]) -> Option<String> {
    let ext = extension_for(content_type)?;
    let digest = Sha256::digest(bytes);
    Some(format!("{}.{}", encode_hex(&digest), ext))
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_a_64_char_hex_digest_plus_the_canonical_extension() {
        let key = media_key("image/png", b"some bytes").unwrap();
        let (hash, ext) = key.split_once('.').unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(ext, "png");
    }

    /// The known SHA-256 of the empty input, so the digest is pinned to the real
    /// algorithm rather than merely "something 64 chars long".
    #[test]
    fn hashes_with_sha256() {
        let key = media_key("image/png", b"").unwrap();
        assert_eq!(
            key,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855.png"
        );
    }

    #[test]
    fn identical_bytes_give_identical_keys() {
        assert_eq!(
            media_key("image/png", b"same"),
            media_key("image/png", b"same")
        );
    }

    #[test]
    fn different_bytes_give_different_keys() {
        assert_ne!(media_key("image/png", b"a"), media_key("image/png", b"b"));
    }

    /// The same bytes stored under different types must not collide, or serving
    /// one key could hand back the other's `Content-Type`.
    #[test]
    fn type_is_part_of_the_key() {
        let png = media_key("image/png", b"same").unwrap();
        let jpeg = media_key("image/jpeg", b"same").unwrap();
        assert_ne!(png, jpeg);
    }

    #[test]
    fn refuses_unsupported_types() {
        assert_eq!(media_key("application/zip", b"bytes"), None);
        assert_eq!(media_key("image/svg+xml", b"bytes"), None);
    }

    /// Keys are used directly as path segments, so nothing that could escape a
    /// directory may appear in one.
    #[test]
    fn contains_no_path_separators_or_traversal() {
        let key = media_key("video/webm", b"bytes").unwrap();
        assert!(!key.contains('/'));
        assert!(!key.contains('\\'));
        assert!(!key.contains(".."));
    }
}
