//! The canonical file extension for a supported MIME type — and the allowlist.

/// The canonical file extension for `content_type`, or `None` if the type is not
/// one we accept.
///
/// This function *is* the upload allowlist: every other type decision in the
/// system reduces to it, so there is exactly one list to audit and no way for a
/// new caller to admit a type the rest of the pipeline doesn't know about.
/// [`is_allowed`](crate::is_allowed) is a thin predicate over it, and
/// [`media_key`](crate::media_key) refuses to mint a key for anything it rejects.
///
/// The client's declared type is untrusted input, so it is normalised first: any
/// `; charset=…`-style parameters are stripped and the result is lower-cased and
/// trimmed. That means `"IMAGE/PNG; charset=utf-8"` resolves like `"image/png"`
/// — a mislabelled *type* is caught later by
/// [`upload_matches_bytes`](crate::upload_matches_bytes), which checks the bytes
/// themselves; this step only stops trivial formatting differences from being
/// mistaken for unsupported types.
///
/// The mapping is deliberately many-to-one with
/// [`content_type_for`](crate::content_type_for): `image/jpeg` normalises to the
/// single extension `jpg`, while *that* function also accepts `jpeg`, so a key
/// minted here always round-trips back to its canonical type.
///
/// Note the absence of SVG: it is a document format that can carry script, and
/// serving one from our own origin would give an uploader script execution there.
pub fn extension_for(content_type: &str) -> Option<&'static str> {
    // Untrusted input: drop any `;parameters`, trim, and case-fold before matching.
    let ct = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    match ct.as_str() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "video/mp4" => Some("mp4"),
        "video/webm" => Some("webm"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_every_supported_type() {
        assert_eq!(extension_for("image/png"), Some("png"));
        assert_eq!(extension_for("image/jpeg"), Some("jpg"));
        assert_eq!(extension_for("image/gif"), Some("gif"));
        assert_eq!(extension_for("image/webp"), Some("webp"));
        assert_eq!(extension_for("video/mp4"), Some("mp4"));
        assert_eq!(extension_for("video/webm"), Some("webm"));
    }

    #[test]
    fn normalises_case_and_parameters() {
        assert_eq!(extension_for("IMAGE/PNG"), Some("png"));
        assert_eq!(extension_for("image/png; charset=utf-8"), Some("png"));
        assert_eq!(extension_for("  image/jpeg  "), Some("jpg"));
    }

    #[test]
    fn rejects_unsupported_types() {
        assert_eq!(extension_for("application/zip"), None);
        assert_eq!(extension_for("text/html"), None);
        assert_eq!(extension_for(""), None);
    }

    #[test]
    fn refuses_svg_because_it_can_carry_script() {
        assert_eq!(extension_for("image/svg+xml"), None);
    }
}
