//! Identifies an invite request (the waitlist entry).

use serde::{Deserialize, Serialize};

/// Identifies an invite request — one entry on the access waitlist. Node-local:
/// the waitlist is not federated, so this id is only meaningful on the node that
/// issued it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct InviteId(pub u64);

impl std::fmt::Display for InviteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
