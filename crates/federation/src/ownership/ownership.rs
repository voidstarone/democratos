//! Who owns a community, and under which epoch.

use domain::NodeId;

/// Who owns a community, and under which epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ownership {
    pub demos: u64,
    pub owner: NodeId,
    pub epoch: u64,
}
