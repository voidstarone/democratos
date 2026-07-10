//! Persistence for governance proposals.

use async_trait::async_trait;

use domain::{DemosId, Proposal, ProposalId, ProposalKind, Timestamp, UserId};

use crate::Result;

#[async_trait]
pub trait ProposalStore: Send + Sync {
    async fn create(
        &self,
        demos: DemosId,
        proposer: UserId,
        kind: ProposalKind,
        opened_at: Timestamp,
        closes_at: Timestamp,
    ) -> Result<Proposal>;
    async fn get(&self, id: ProposalId) -> Result<Option<Proposal>>;
    async fn update(&self, proposal: &Proposal) -> Result<()>;
    async fn list(&self, demos: DemosId) -> Result<Vec<Proposal>>;
}
