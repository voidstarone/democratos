//! Mask selecting the sequence portion of a composite ID.

use crate::SEQUENCE_BITS;

/// Mask selecting the sequence portion of a composite ID.
pub const SEQUENCE_MASK: u64 = (1u64 << SEQUENCE_BITS) - 1;
