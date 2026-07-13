//! Per-node configuration for the media-ingest safety pipeline.

use app::ScanFailurePolicy;

use crate::sanitizer_kind::SanitizerKind;

/// Everything the composition root needs to build the media-safety guard. Bundled
/// so `build_services` gains one parameter, not five.
pub(crate) struct MediaGuardConfig {
    /// Which sanitizer defends against malicious media.
    pub(crate) sanitizer: SanitizerKind,
    /// Whether to run the CSAM scanner at all (vs. an explicit opt-out).
    pub(crate) csam_scan: bool,
    /// Path to the operator-curated known-bad hash corpus, if any.
    pub(crate) hash_file: Option<String>,
    /// What to do when the scanner cannot decide.
    pub(crate) policy: ScanFailurePolicy,
    /// Directory blocked/held uploads are preserved in.
    pub(crate) quarantine_dir: String,
}
