//! Drives failover from the perspective of one node.

use domain::NodeId;

use crate::{ClaimOutcome, OwnershipRegistry, RegistryError};

use super::choose_new_owner::choose_new_owner;
use super::choose_new_standby::choose_new_standby;
use super::rehome_outcome::RehomeOutcome;

/// Drives failover from the perspective of one node. Run its [`tick`](Self::tick) on
/// an interval over the communities this node replicates.
pub struct RehomingController {
    node: NodeId,
    registry: std::sync::Arc<dyn OwnershipRegistry>,
}

impl RehomingController {
    pub fn new(node: NodeId, registry: std::sync::Arc<dyn OwnershipRegistry>) -> Self {
        Self { node, registry }
    }

    /// Evaluate every candidate community once, taking over the ones this node is
    /// the best standby for. Registry errors on a single community are skipped
    /// (logged by the caller via the returned outcomes being short).
    pub async fn tick(&self, candidates: &[u64]) -> Vec<RehomeOutcome> {
        let mut out = Vec::new();
        for &demos in candidates {
            if let Ok(o) = self.consider(demos).await {
                out.push(o);
            }
        }
        out
    }

    async fn consider(&self, demos: u64) -> Result<RehomeOutcome, RegistryError> {
        if self.registry.owner_of(demos).await?.is_some() {
            return Ok(RehomeOutcome::StillOwned { demos });
        }
        let standbys = self.registry.standbys(demos).await?;
        let loads = self.registry.live_nodes().await?;

        let Some(winner) = choose_new_owner(&standbys, &loads) else {
            return Ok(RehomeOutcome::Stranded { demos });
        };
        if winner != self.node {
            return Ok(RehomeOutcome::Yielded { demos, to: winner });
        }

        // We are the best candidate — claim (bumps the epoch, fencing the old owner).
        match self.registry.claim(demos, self.node).await? {
            ClaimOutcome::Claimed { epoch } => {
                // Re-protect the community with a fresh, quiet standby.
                if let Some(sb) = choose_new_standby(&[self.node], &loads) {
                    let _ = self.registry.set_standby(demos, sb).await;
                }
                Ok(RehomeOutcome::Promoted { demos, epoch })
            }
            // Lost a race to another node between our read and our claim.
            ClaimOutcome::Held { by, .. } => Ok(RehomeOutcome::Yielded { demos, to: by }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InMemoryRegistry, NodeLoad};

    #[tokio::test]
    async fn failover_promotes_the_quiet_standby_and_fences_the_old_owner() {
        let reg = std::sync::Arc::new(InMemoryRegistry::new());
        // Node 1 owns d/7; nodes 2 and 3 are standbys.
        reg.claim(7, NodeId(1)).await.unwrap();
        reg.set_standby(7, NodeId(2)).await.unwrap();
        reg.set_standby(7, NodeId(3)).await.unwrap();
        // Live loads for the standbys only (node 1 is about to go "down").
        reg.report_load(
            NodeId(2),
            NodeLoad {
                hosted_communities: 5,
                requests_per_sec: 10.0,
            },
        )
        .await
        .unwrap();
        reg.report_load(
            NodeId(3),
            NodeLoad {
                hosted_communities: 1,
                requests_per_sec: 2.0,
            },
        )
        .await
        .unwrap();

        // Node 1 goes down → its lease lapses → unowned.
        reg.release(7, NodeId(1)).await.unwrap();

        // The busy standby (2) yields to the quiet one (3).
        let c2 = RehomingController::new(NodeId(2), reg.clone());
        assert_eq!(
            c2.tick(&[7]).await,
            vec![RehomeOutcome::Yielded {
                demos: 7,
                to: NodeId(3)
            }]
        );

        // The quiet standby (3) promotes itself; the epoch bumps past 1.
        let c3 = RehomingController::new(NodeId(3), reg.clone());
        let outcomes = c3.tick(&[7]).await;
        assert!(
            matches!(outcomes[0], RehomeOutcome::Promoted { demos: 7, epoch } if epoch > 1),
            "got {outcomes:?}"
        );
        let owner = reg.owner_of(7).await.unwrap().unwrap();
        assert_eq!(owner.owner, NodeId(3));

        // A fresh standby was designated (the quiet remaining node, 2).
        assert!(reg.standbys(7).await.unwrap().contains(&NodeId(2)));

        // The old owner (1) returning is fenced: it cannot reclaim.
        assert!(matches!(
            reg.claim(7, NodeId(1)).await.unwrap(),
            ClaimOutcome::Held { by, .. } if by == NodeId(3)
        ));

        // A now-owned community is left alone.
        assert_eq!(
            c3.tick(&[7]).await,
            vec![RehomeOutcome::StillOwned { demos: 7 }]
        );
    }
}
