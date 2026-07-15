//! Persistence for trials by jury.

use async_trait::async_trait;

use domain::{DemosId, ReportId, Timestamp, Trial, TrialId, UserId};

use crate::Result;

#[async_trait]
pub trait TrialStore: Send + Sync {
    async fn create(
        &self,
        demos: DemosId,
        report: ReportId,
        accused: UserId,
        jurors: Vec<UserId>,
        jury_weight: u64,
        // Each juror's frozen vote weight, aligned by index with `jurors`. Sums to
        // `jury_weight`; empty (or all-1) under one-juror-one-vote.
        juror_weights: Vec<u64>,
        opened_at: Timestamp,
        closes_at: Timestamp,
    ) -> Result<Trial>;
    async fn get(&self, id: TrialId) -> Result<Option<Trial>>;
    async fn update(&self, trial: &Trial) -> Result<()>;
    async fn list_open(&self, demos: DemosId) -> Result<Vec<Trial>>;
    /// Every trial in `demos`, open and settled, newest first, capped to a recent
    /// window. Backs the community's public case log; trials are public record.
    async fn list_for_demos(&self, demos: DemosId) -> Result<Vec<Trial>>;
    /// Record a juror's ballot carrying their `weight` (1 under one-juror-one-vote).
    /// Errors with `AlreadyVoted` on a repeat.
    async fn cast_ballot(
        &self,
        trial: TrialId,
        juror: UserId,
        guilty: bool,
        weight: u64,
    ) -> Result<()>;
    async fn has_voted(&self, trial: TrialId, juror: UserId) -> Result<bool>;
    /// Returns (guilty, not_guilty) totals, summed by ballot weight.
    async fn ballot_tally(&self, trial: TrialId) -> Result<(u64, u64)>;
}
