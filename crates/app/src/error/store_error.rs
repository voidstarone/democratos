//! The storage vocabulary: the errors every `*Store` port and its adapters may
//! emit, and the default error of the crate's [`Result`](crate::Result) alias.

use thiserror::Error;

/// Failures that arise from persistence. Adapters map their internal errors (SQL
/// errors, IO errors, …) into [`StoreError::Store`]; the other variants are the
/// structured outcomes a store distinguishes (missing row, optimistic-concurrency
/// conflict, uniqueness violation, a duplicate ballot).
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("storage failure: {0}")]
    Store(String),

    #[error("not found")]
    NotFound,

    /// An optimistic-concurrency conflict: the row changed between the read and the
    /// write (another replica updated it first), so the write was refused rather
    /// than silently overwriting the newer state. The caller may re-read and retry.
    #[error("the record was modified concurrently; retry")]
    Conflict,

    #[error("a record with that identity already exists")]
    AlreadyExists,

    #[error("this voter has already cast a ballot on the proposal")]
    AlreadyVoted,
}
