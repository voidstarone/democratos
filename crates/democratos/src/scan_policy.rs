//! CLI mapping for the scan-failure policy.

use app::ScanFailurePolicy;
use clap::ValueEnum;

/// How a node behaves when the CSAM scanner cannot render a verdict. Mirrors
/// [`ScanFailurePolicy`] for the command line (a positive match is always blocked,
/// whatever this says).
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum ScanPolicy {
    /// Refuse the upload — nothing unscanned is stored (the default).
    FailClosed,
    /// Refuse and preserve a copy in quarantine for review; never serve it.
    Quarantine,
    /// Accept and serve, logging that the upload went unscanned.
    Allow,
}

impl ScanPolicy {
    pub(crate) fn to_app(self) -> ScanFailurePolicy {
        match self {
            ScanPolicy::FailClosed => ScanFailurePolicy::FailClosed,
            ScanPolicy::Quarantine => ScanFailurePolicy::Quarantine,
            ScanPolicy::Allow => ScanFailurePolicy::Allow,
        }
    }
}
