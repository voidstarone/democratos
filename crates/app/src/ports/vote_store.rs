//! Persistence for governance proposal ballots.

use async_trait::async_trait;

use domain::{ProposalId, Tally, Timestamp, UserId};

use crate::Result;

#[async_trait]
pub trait VoteStore: Send + Sync {
    /// Record a ballot carrying the voter's `weight` (1 under one-person-one-vote).
    /// Errors with `AlreadyVoted` if this voter already voted.
    async fn cast(
        &self,
        proposal: ProposalId,
        voter: UserId,
        aye: bool,
        weight: u64,
        at: Timestamp,
    ) -> Result<()>;
    async fn has_voted(&self, proposal: ProposalId, voter: UserId) -> Result<bool>;
    /// Aye/nay totals, summed by ballot weight.
    async fn tally(&self, proposal: ProposalId) -> Result<Tally>;
}
