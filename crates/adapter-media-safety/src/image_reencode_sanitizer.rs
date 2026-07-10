//! Decodes, bounds, and re-encodes uploaded images; validates video.

use std::io::Cursor;

use async_trait::async_trait;
use image::{DynamicImage, ImageFormat, ImageReader, Limits};

use app::{MediaError, MediaSanitizer, SanitizedMedia, MAX_IMAGE_DIMENSION, MAX_IMAGE_PIXELS};

use crate::validate_video::validate_video;

/// The default [`MediaSanitizer`]: it turns an untrusted upload into bytes safe to
/// store and serve.
///
/// For **images** it reads the dimensions from the header and rejects a
/// decompression/pixel bomb *before* decoding a single pixel, decodes with hard
/// allocation limits (so a malformed file cannot exhaust memory), and — for PNG
/// and JPEG — re-encodes from the decoded pixels, so what we persist carries none
/// of the original's metadata (EXIF/GPS), trailing payload, or polyglot framing.
/// Animated **GIF/WebP** are validated by decoding but kept byte-for-byte, because
/// re-encoding would flatten them to a single frame; the dimension/decode checks
/// still defuse bombs and malformed files.
///
/// For **video** (mp4/webm) it structurally validates the container without
/// transcoding (see [`validate_video`]).
#[derive(Default)]
pub struct ImageReencodeSanitizer;

#[async_trait]
impl MediaSanitizer for ImageReencodeSanitizer {
    async fn sanitize(
        &self,
        declared_content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<SanitizedMedia, MediaError> {
        // Strip any `; charset=…` and lower-case for matching.
        let ct = declared_content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        match ct.as_str() {
            "image/png" => reencode(&bytes, ImageFormat::Png, "image/png"),
            "image/jpeg" => reencode(&bytes, ImageFormat::Jpeg, "image/jpeg"),
            // Animation-preserving: validate by decoding the first frame, keep bytes.
            "image/gif" => validate_only(&bytes, ImageFormat::Gif, "image/gif"),
            "image/webp" => validate_only(&bytes, ImageFormat::WebP, "image/webp"),
            "video/mp4" | "video/webm" => {
                let canonical = validate_video(&ct, &bytes)?;
                Ok(SanitizedMedia::new(canonical, bytes))
            }
            other => Err(MediaError::Rejected(format!(
                "unsupported upload type: {other}"
            ))),
        }
    }
}

/// Reject a decompression/pixel bomb from the header, then decode with allocation
/// limits, returning the decoded image. Shared by both image paths.
fn decode_bounded(bytes: &[u8], format: ImageFormat) -> Result<DynamicImage, MediaError> {
    // Dimensions come from the header alone — no pixel buffer is allocated yet.
    let (w, h) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| MediaError::Rejected("that image could not be read".to_string()))?;
    if w > MAX_IMAGE_DIMENSION || h > MAX_IMAGE_DIMENSION {
        return Err(MediaError::Rejected(
            "that image's dimensions are too large".to_string(),
        ));
    }
    if u64::from(w) * u64::from(h) > MAX_IMAGE_PIXELS {
        return Err(MediaError::Rejected(
            "that image has too many pixels".to_string(),
        ));
    }
    // Belt-and-braces: cap the decoder's own allocation too.
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(MAX_IMAGE_PIXELS.saturating_mul(4));
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|_| MediaError::Rejected("that image could not be decoded".to_string()))
}

/// Decode (proving the bytes are valid, bounded media) then re-encode from pixels,
/// discarding all original framing and metadata.
fn reencode(
    bytes: &[u8],
    format: ImageFormat,
    canonical_ct: &str,
) -> Result<SanitizedMedia, MediaError> {
    let img = decode_bounded(bytes, format)?;
    // JPEG cannot carry an alpha channel; drop it so encoding can't fail on RGBA.
    let img = if format == ImageFormat::Jpeg {
        DynamicImage::ImageRgb8(img.to_rgb8())
    } else {
        img
    };
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), format)
        .map_err(|e| MediaError::Store(format!("re-encode failed: {e}")))?;
    Ok(SanitizedMedia::new(canonical_ct, out))
}

/// Decode to validate (bounds + real format) but keep the original bytes, so an
/// animation survives. Used for GIF/WebP.
fn validate_only(
    bytes: &[u8],
    format: ImageFormat,
    canonical_ct: &str,
) -> Result<SanitizedMedia, MediaError> {
    decode_bounded(bytes, format)?;
    Ok(SanitizedMedia::new(canonical_ct, bytes.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, RgbImage};

    fn png_bytes(w: u32, h: u32) -> Vec<u8> {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(w, h, image::Rgb([10, 20, 30])));
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png).unwrap();
        out
    }

    #[tokio::test]
    async fn valid_png_is_reencoded_to_png() {
        let s = ImageReencodeSanitizer;
        let clean = s.sanitize("image/png", png_bytes(8, 8)).await.unwrap();
        assert_eq!(clean.content_type, "image/png");
        // Still a decodable PNG after the round-trip.
        assert!(ImageReader::with_format(Cursor::new(&clean.bytes), ImageFormat::Png)
            .decode()
            .is_ok());
    }

    #[tokio::test]
    async fn a_document_declared_as_png_is_rejected() {
        let s = ImageReencodeSanitizer;
        let html = b"<!doctype html><script>alert(1)</script>".to_vec();
        let err = s.sanitize("image/png", html).await.unwrap_err();
        assert!(matches!(err, MediaError::Rejected(_)));
    }

    #[tokio::test]
    async fn png_reencode_strips_trailing_payload() {
        let s = ImageReencodeSanitizer;
        // A valid PNG with junk appended (a classic polyglot shape).
        let mut poly = png_bytes(8, 8);
        let original_len = poly.len();
        poly.extend_from_slice(b"TRAILING-EVIL-PAYLOAD-XXXXXXXXXXXXXXXX");
        let clean = s.sanitize("image/png", poly).await.unwrap();
        // The re-encoded output is derived only from decoded pixels, so the
        // appended bytes cannot survive.
        assert!(!clean
            .bytes
            .windows(5)
            .any(|w| w == b"EVIL-"));
        // (Length differs from the tampered input.)
        assert_ne!(clean.bytes.len(), original_len + 37);
    }

    #[tokio::test]
    async fn unsupported_type_is_rejected() {
        let s = ImageReencodeSanitizer;
        let err = s.sanitize("application/zip", vec![1, 2, 3]).await.unwrap_err();
        assert!(matches!(err, MediaError::Rejected(_)));
    }
}
