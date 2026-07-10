//! Matches media against a curated known-bad hash corpus.

use std::io::Cursor;

use async_trait::async_trait;
use image::{ImageFormat, ImageReader};
use sha2::{Digest, Sha256};

use app::{MediaError, MediaSafetyScanner, SafetyVerdict};

use crate::known_hash_set::KnownHashSet;
use crate::perceptual_hash::dhash;

/// A [`MediaSafetyScanner`] that matches uploads against a locally curated
/// [`KnownHashSet`] — the honest, offline baseline for known-CSAM detection.
///
/// It checks two ways: a **cryptographic** SHA-256 match (a byte-identical copy)
/// and, for images, a **perceptual** dHash match (a resized/recompressed copy).
/// It cannot detect *novel* material — for that a deployment wires an external
/// classifier at this same port. Because the corpus is local it is always
/// available, so this scanner never reports "unavailable"; the fail-closed /
/// quarantine policy exists for an external adapter that can.
///
/// An **empty** corpus matches nothing: the scanner then clears everything, which
/// is why the composition root warns loudly when no hash file is configured — the
/// CSAM check is effectively off until an operator supplies one.
pub struct HashListSafetyScanner {
    corpus: KnownHashSet,
    source: String,
}

impl HashListSafetyScanner {
    pub fn new(corpus: KnownHashSet, source: impl Into<String>) -> Self {
        Self {
            corpus,
            source: source.into(),
        }
    }

    /// Whether the backing corpus is empty (the scan is a no-op).
    pub fn is_noop(&self) -> bool {
        self.corpus.is_empty()
    }
}

#[async_trait]
impl MediaSafetyScanner for HashListSafetyScanner {
    async fn scan(&self, content_type: &str, bytes: &[u8]) -> Result<SafetyVerdict, MediaError> {
        // Exact match first — cheap and format-independent.
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        if self.corpus.contains_sha256(&digest) {
            return Ok(SafetyVerdict::matched(self.source.clone(), "sha256"));
        }

        // Perceptual match for images that decode. A decode failure here is not a
        // safety error — the sanitizer already vouched for the bytes — so we skip
        // perceptual matching rather than reporting the scanner unavailable.
        if let Some(format) = image_format(content_type) {
            if let Ok(img) = ImageReader::with_format(Cursor::new(bytes), format).decode() {
                if self.corpus.matches_perceptual(dhash(&img)) {
                    return Ok(SafetyVerdict::matched(self.source.clone(), "perceptual"));
                }
            }
        }

        Ok(SafetyVerdict::Clear)
    }
}

fn image_format(content_type: &str) -> Option<ImageFormat> {
    match content_type {
        "image/png" => Some(ImageFormat::Png),
        "image/jpeg" => Some(ImageFormat::Jpeg),
        "image/gif" => Some(ImageFormat::Gif),
        "image/webp" => Some(ImageFormat::WebP),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage};

    fn png(v: u8) -> Vec<u8> {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(8, 8, image::Rgb([v, v, v])));
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png).unwrap();
        out
    }

    #[tokio::test]
    async fn empty_corpus_clears_everything() {
        let s = HashListSafetyScanner::new(KnownHashSet::empty(), "test");
        assert!(s.is_noop());
        assert_eq!(
            s.scan("image/png", &png(1)).await.unwrap(),
            SafetyVerdict::Clear
        );
    }

    #[tokio::test]
    async fn exact_sha256_is_matched() {
        let bytes = png(2);
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let corpus = KnownHashSet::parse(&format!("sha256:{hex}\n"));
        let s = HashListSafetyScanner::new(corpus, "test-list");
        assert!(s.scan("image/png", &bytes).await.unwrap().is_match());
        // A different image is untouched.
        assert_eq!(
            s.scan("image/png", &png(200)).await.unwrap(),
            SafetyVerdict::Clear
        );
    }

    #[tokio::test]
    async fn perceptual_near_duplicate_is_matched() {
        let bytes = png(3);
        let img = ImageReader::with_format(Cursor::new(&bytes), ImageFormat::Png)
            .decode()
            .unwrap();
        let fp = dhash(&img);
        let corpus = KnownHashSet::parse(&format!("dhash:{fp:016x}\n"));
        let s = HashListSafetyScanner::new(corpus, "test-list");
        assert!(s.scan("image/png", &bytes).await.unwrap().is_match());
    }
}
