//! A live node and its last-reported load.

use domain::NodeId;

use crate::NodeLoad;

/// A live node and its last-reported load.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeStatus {
    pub node: NodeId,
    pub load: NodeLoad,
}
