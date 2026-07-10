//! The result of trying to claim a community.

use domain::NodeId;

/// The result of trying to [`OwnershipRegistry::claim`](crate::OwnershipRegistry::claim)
/// a community.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimOutcome {
    /// This node now owns the community, at the freshly bumped `epoch`.
    Claimed { epoch: u64 },
    /// Another live node already holds it; not claimed.
    Held { by: NodeId, epoch: u64 },
}
