//! A safety scanner that clears everything — for when scanning is off.

use async_trait::async_trait;

use app::{MediaError, MediaSafetyScanner, SafetyVerdict};

/// A [`MediaSafetyScanner`] that clears every upload. Wired when CSAM scanning is
/// explicitly disabled on a node, leaving the pipeline as sanitize-only. It is a
/// *deliberate* opt-out — distinct from an empty hash list, which is an accidental
/// no-op the composition root warns about — so the choice to run without scanning
/// is explicit in the wiring, not a silent default.
#[derive(Default)]
pub struct AllowAllSafetyScanner;

#[async_trait]
impl MediaSafetyScanner for AllowAllSafetyScanner {
    async fn scan(&self, _content_type: &str, _bytes: &[u8]) -> Result<SafetyVerdict, MediaError> {
        Ok(SafetyVerdict::Clear)
    }
}
