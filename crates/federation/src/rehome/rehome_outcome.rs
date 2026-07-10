//! What one rehoming evaluation of a community concluded.

use domain::NodeId;

/// What one rehoming evaluation of a community concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehomeOutcome {
    /// Still has a live owner — nothing to do.
    StillOwned { demos: u64 },
    /// This node took over as the new owner at `epoch`.
    Promoted { demos: u64, epoch: u64 },
    /// Unowned, but another node is the better (quieter) candidate — leave it.
    Yielded { demos: u64, to: NodeId },
    /// Unowned and no live standby can take it (operator attention needed).
    Stranded { demos: u64 },
}
