//! The default jury size.

/// Default jury size; the application caps it at the number of eligible members.
///
/// Retained for callers that empanel a fixed panel; demoi that size their juries
/// by policy use [`crate::JurySizing`] instead.
pub const DEFAULT_JURY_SIZE: usize = 7;
