//! The largest local sequence a single node can mint.

use crate::SEQUENCE_MASK;

/// The largest local sequence a single node can mint (2^48 − 1).
pub const MAX_SEQUENCE: u64 = SEQUENCE_MASK;
