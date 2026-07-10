//! Scans media bytes for known illegal content (CSAM) before they are stored.

use async_trait::async_trait;

use crate::ports::safety_verdict::SafetyVerdict;
use crate::MediaError;

/// Scans media bytes for known illegal content — specifically child sexual abuse
/// material — before they are ever written to the store a CDN serves from.
///
/// There is no honest way to do this with a local heuristic: effective detection
/// is either a match against a curated corpus of known-bad hashes (cryptographic
/// for exact copies, perceptual for near-duplicates — e.g. NCMEC / PhotoDNA hash
/// sets, access to which is legally gated) or an external classifier for novel
/// material (Thorn Safer, Google Content Safety). This port is the seam those
/// plug into. The bundled adapter matches a locally curated hash list; a
/// deployment enrols with a provider and wires an HTTP adapter here.
///
/// Contract:
/// * `Ok(SafetyVerdict::Clear)` — no known-bad entry matched. **Not** a claim that
///   the media is safe, only that the scanner has nothing on it.
/// * `Ok(SafetyVerdict::Match { .. })` — a positive hit; the pipeline blocks and
///   preserves the bytes.
/// * `Err(_)` — the scanner could not run (backend down, not configured). The
///   ingest policy — not the scanner — decides whether that fails the upload
///   closed, so implementations must surface unavailability as an error and must
///   never downgrade it to `Clear`.
#[async_trait]
pub trait MediaSafetyScanner: Send + Sync {
    /// Scan the (already sanitized) `bytes` of `content_type`.
    async fn scan(&self, content_type: &str, bytes: &[u8]) -> Result<SafetyVerdict, MediaError>;
}
