//! Recover the per-node sequence from a composite ID.

use crate::SEQUENCE_MASK;

/// Recover the per-node sequence encoded in `id`.
#[inline]
pub const fn local_sequence(id: u64) -> u64 {
    id & SEQUENCE_MASK
}
