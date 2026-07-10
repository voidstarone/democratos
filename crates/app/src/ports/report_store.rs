//! Persistence for moderation reports.

use async_trait::async_trait;

use domain::{DemosId, Report, ReportId, ReportReason, ReportTarget, Timestamp, UserId};

use crate::Result;

#[async_trait]
pub trait ReportStore: Send + Sync {
    async fn create(
        &self,
        demos: DemosId,
        reporter: Option<UserId>,
        target: ReportTarget,
        reason: ReportReason,
        note: &str,
        at: Timestamp,
    ) -> Result<Report>;
    async fn get(&self, id: ReportId) -> Result<Option<Report>>;
    async fn update(&self, report: &Report) -> Result<()>;
    async fn list_open(&self, demos: DemosId) -> Result<Vec<Report>>;
}
