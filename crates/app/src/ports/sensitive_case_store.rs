//! Persistence for platform-wide sensitive-content review cases.

use async_trait::async_trait;

use domain::{ReportTarget, SensitiveCase, SensitiveCaseId, Timestamp, UserId};

use crate::Result;

/// Persistence for [`SensitiveCase`]s — the platform-wide (extra-demos) review
/// queue. Mirrors [`ReportStore`](crate::ReportStore) but is **not** scoped to a
/// demos: sensitive/illegal content is reviewed by the platform-wide reviewer
/// pool, not a community jury.
#[async_trait]
pub trait SensitiveCaseStore: Send + Sync {
    /// Open a new case for `target`, flagged by `reporter`.
    async fn create(
        &self,
        reporter: Option<UserId>,
        target: ReportTarget,
        note: &str,
        at: Timestamp,
    ) -> Result<SensitiveCase>;
    async fn get(&self, id: SensitiveCaseId) -> Result<Option<SensitiveCase>>;
    /// The open case for a given target, if one exists (a new flag merges into it).
    async fn open_for_target(&self, target: ReportTarget) -> Result<Option<SensitiveCase>>;
    async fn update(&self, case: &SensitiveCase) -> Result<()>;
    /// All cases still gathering reviewer classifications — the review queue.
    async fn list_open(&self) -> Result<Vec<SensitiveCase>>;
    /// How many cases are open — backs the reviewer nav badge.
    async fn count_open(&self) -> Result<u64>;
}
