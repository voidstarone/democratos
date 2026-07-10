//! Preserves blocked media out of reach of the public store.

use async_trait::async_trait;

use crate::MediaError;

/// Preserves media that the pipeline has refused — a known-bad (CSAM) match, or an
/// upload that could not be scanned under a policy that requires holding it — in a
/// restricted location the public store and CDN never read from.
///
/// This is a legal-preservation seam, not a bin. In the United States a provider
/// that becomes aware of apparent CSAM must report it to the NCMEC CyberTipline
/// and **preserve** the content (18 U.S.C. §2258A) — deleting it destroys evidence
/// the law requires be kept. So the pipeline never discards a blocked upload; it
/// hands the bytes here, where they are written with restrictive permissions and
/// recorded in an incident log for an operator to action. Implementations must not
/// serve, index, or expose what they hold.
#[async_trait]
pub trait MediaQuarantine: Send + Sync {
    /// Preserve `bytes` of `content_type`, tagging the record with why it was held
    /// (`reason`). Returns an opaque incident id for logs and follow-up. Failing to
    /// preserve is an error the caller must treat as fatal to the upload — never a
    /// signal to fall through and serve the bytes anyway.
    async fn preserve(
        &self,
        content_type: &str,
        bytes: &[u8],
        reason: &str,
    ) -> Result<String, MediaError>;
}
