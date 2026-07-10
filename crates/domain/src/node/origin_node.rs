//! Recover the origin node from a composite ID.

use crate::{NodeId, SEQUENCE_BITS};

/// Recover the origin node — the node that minted `id`.
#[inline]
pub const fn origin_node(id: u64) -> NodeId {
    NodeId((id >> SEQUENCE_BITS) as u16)
}
