//! Facade delegators for governance use-cases. The logic now lives in
//! [`GovernanceService`](super::governance_service::GovernanceService); these thin
//! methods keep `services.open_proposal()` and friends working for call sites not
//! yet migrated off the `Services` aggregator.

use std::sync::Arc;

use domain::{DemosId, Proposal, ProposalId, ProposalKind, ProposalStatus, Rule, UserId};

use crate::{CastVoteError, CloseProposalError, OpenProposalError, Result};

use super::governance_service::GovernanceService;
use super::services::Services;

impl Services {
    /// Build the extracted [`GovernanceService`] from the ports this aggregator
    /// still holds, wiring its account, moderation, and membership peers inline.
    /// Cheap — `Arc` clones only — so delegators construct one per call rather than
    /// storing a field (which would break every `Services { … }` literal). Removed
    /// once all call sites inject `GovernanceService` directly.
    pub(super) fn governance_service(&self) -> GovernanceService {
        GovernanceService::new(
            self.proposals.clone(),
            self.votes.clone(),
            self.rules.clone(),
            self.demoi.clone(),
            self.memberships.clone(),
            self.clock.clone(),
            Arc::new(self.account_service()),
            Arc::new(self.moderation_service()),
            Arc::new(self.membership_service()),
        )
    }

    pub async fn open_proposal(
        &self,
        proposer: UserId,
        demos: DemosId,
        kind: ProposalKind,
    ) -> Result<Proposal, OpenProposalError> {
        self.governance_service()
            .open_proposal(proposer, demos, kind)
            .await
    }

    pub async fn cast_vote(
        &self,
        proposal: ProposalId,
        voter: UserId,
        aye: bool,
        sig: Option<&str>,
    ) -> Result<(), CastVoteError> {
        self.governance_service()
            .cast_vote(proposal, voter, aye, sig)
            .await
    }

    /// Tally and close a proposal, applying the phase-appropriate threshold and
    /// (for constitutional changes) the timelock. A constitutional change that
    /// has already passed its recall window is applied to the demos's criteria.
    pub async fn close_proposal(
        &self,
        proposal: ProposalId,
    ) -> Result<ProposalStatus, CloseProposalError> {
        self.governance_service().close_proposal(proposal).await
    }

    pub async fn list_rules(&self, demos: DemosId) -> Result<Vec<Rule>> {
        self.governance_service().list_rules(demos).await
    }
}
