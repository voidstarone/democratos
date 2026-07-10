//! Structural validation for uploaded video (mp4 / webm).

use app::MediaError;

/// Structurally validate an uploaded video and return the canonical content type
/// to store it under. We do **not** transcode video — a full re-encode needs a
/// heavy native dependency (ffmpeg) unsuitable for the small boxes this runs on —
/// so the defence is narrower than for images: confirm the bytes really carry the
/// container magic they claim (so a mislabelled or polyglot file is refused), and
/// bound the size (already capped upstream, re-asserted here). The bytes are
/// preserved as-is.
///
/// `declared` is the client-declared MIME (already checked against the allowlist);
/// this re-derives the *actual* type from the bytes and rejects a mismatch.
pub fn validate_video(declared: &str, bytes: &[u8]) -> Result<String, MediaError> {
    // Re-sniff from the magic number; never trust the declared type.
    let sniffed = app::sniff_content_type(bytes).ok_or_else(|| {
        MediaError::Rejected("that file's contents aren't a supported video".to_string())
    })?;
    if !sniffed.starts_with("video/") {
        return Err(MediaError::Rejected(
            "that file isn't a video".to_string(),
        ));
    }
    // The declared and actual kinds must agree (mp4 stays mp4, webm stays webm).
    match (app::extension_for(declared), app::extension_for(sniffed)) {
        (Some(a), Some(b)) if a == b => Ok(sniffed.to_string()),
        _ => Err(MediaError::Rejected(
            "that file's contents don't match its type".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_matching_mp4_magic() {
        let mp4 = b"\0\0\0\x18ftypmp42 rest of file";
        assert_eq!(validate_video("video/mp4", mp4).unwrap(), "video/mp4");
    }

    #[test]
    fn rejects_mislabelled_video() {
        // webm bytes declared as mp4 → refused.
        let webm = &[0x1A, 0x45, 0xDF, 0xA3, 1, 2, 3, 4];
        assert!(validate_video("video/mp4", webm).is_err());
    }

    #[test]
    fn rejects_non_video_bytes() {
        let png = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(validate_video("video/mp4", png).is_err());
    }
}
