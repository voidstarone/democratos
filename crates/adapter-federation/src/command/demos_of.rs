use app::{Services, StoreError};
use domain::{DemosId, PostId, ProposalId, TrialId};

use crate::Command;

/// Which community a command targets — read locally (from the replica) to decide
/// routing. The target object's community never changes, so a replica read is safe.
pub async fn demos_of(services: &Services, cmd: &Command) -> Result<DemosId, StoreError> {
    match cmd {
        Command::CastVote { proposal, .. } => Ok(services
            .proposals
            .get(ProposalId(*proposal))
            .await?
            .ok_or(StoreError::NotFound)?
            .demos_id),
        Command::VotePost { post, .. } => Ok(services
            .posts
            .get(PostId(*post))
            .await?
            .ok_or(StoreError::NotFound)?
            .demos_id),
        Command::CastJuryVote { trial, .. } => Ok(services
            .trials
            .get(TrialId(*trial))
            .await?
            .ok_or(StoreError::NotFound)?
            .demos_id),
        // Global account operations have no community to route by — they are directed
        // to a trusted issuer, not a community owner, so must never reach this path.
        Command::MintAccount { .. } => Err(StoreError::Store(
            "MintAccount is not community-scoped; route it to a trusted issuer".into(),
        )),
        Command::Authenticate { .. } => Err(StoreError::Store(
            "Authenticate is not community-scoped; route it to the home issuer".into(),
        )),
    }
}
