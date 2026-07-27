//! What a node does when the media safety scanner cannot render a verdict.

/// What the ingest pipeline does when the [`MediaSafetyScanner`](crate::MediaSafetyScanner)
/// is *unavailable* — not configured, backend down, request failed.
///
/// This governs only the "can't decide" case. A **positive match is always
/// blocked and preserved**, whatever this is set to; there is no policy value
/// that serves known-bad media. The choice here is purely how much availability
/// an operator will trade for the guarantee that nothing unscanned is served.
///
/// Kept in the application layer, not the composition root, because
/// [`GuardedMediaStore`](crate::GuardedMediaStore) is what acts on it; the CLI
/// merely maps its flag onto these variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanFailurePolicy {
    /// Refuse the upload. Nothing unscanned is ever stored. The safe default, and
    /// the right one for a public node: an outage degrades uploads, not safety.
    FailClosed,
    /// Refuse the upload *and* preserve a copy in quarantine for review. Costs
    /// storage and creates material an operator must handle, but keeps evidence
    /// of what was being uploaded while the scanner was blind.
    Quarantine,
    /// Store and serve the upload, recording that it went unscanned. Highest
    /// availability, weakest guarantee — only sensible where the media is
    /// otherwise trusted (a closed, invite-only node).
    Allow,
}
