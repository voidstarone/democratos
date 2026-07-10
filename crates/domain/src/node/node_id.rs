//! A node identity in the federated network.

use serde::{Deserialize, Serialize};

/// Identifies one node (one deployment: its own database + app + media) in the
/// federated network. Node `0` is the reserved single-box / bootstrap identity,
/// so an un-federated deployment keeps minting `1, 2, 3, …` exactly as before.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct NodeId(pub u16);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
