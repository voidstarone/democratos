//! A node's current load, reported to the control plane.

/// A node's current load, reported to the control plane so rehoming can pick the
/// least-loaded target (Phase 6).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeLoad {
    /// Communities this node currently owns.
    pub hosted_communities: u32,
    /// Recent request rate.
    pub requests_per_sec: f64,
}
