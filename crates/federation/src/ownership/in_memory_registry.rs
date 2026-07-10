//! A process-local `OwnershipRegistry` for single-node / dev / tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use domain::NodeId;

use crate::{
    binding_is_authoritative, ClaimOutcome, NodeLoad, NodePublicKey, NodeStatus, Ownership,
    OwnershipRegistry, RegistryError,
};

#[derive(Default)]
struct RegistryState {
    /// demos → (owner, epoch). Epoch is monotonic per demos and survives handoff.
    owners: HashMap<u64, (NodeId, u64)>,
    /// The highest epoch ever assigned to a demos, so a re-claim always bumps.
    max_epoch: HashMap<u64, u64>,
    /// node → public key hex.
    keys: HashMap<u16, String>,
    /// node → last-reported load (presence implies "live" in this simple model).
    loads: HashMap<u16, NodeLoad>,
    /// demos → designated standby nodes.
    standbys: HashMap<u64, Vec<u16>>,
    /// demos → community public key hex (first-write-wins).
    community_keys: HashMap<u64, String>,
    /// demos → its current (highest-epoch) founder-signed home binding.
    home_bindings: HashMap<u64, crate::HomeBinding>,
}

/// A process-local [`OwnershipRegistry`] with no real leases — ownership is
/// explicit ([`claim`](OwnershipRegistry::claim)/[`release`](OwnershipRegistry::release)).
/// It models the epoch-fencing semantics faithfully (a re-claim always bumps past
/// the highest epoch the community ever had), which is what the authorization logic
/// depends on. Good for a single-node deployment and for deterministic tests;
/// production uses the etcd adapter.
#[derive(Default)]
pub struct InMemoryRegistry {
    state: Mutex<RegistryState>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl OwnershipRegistry for InMemoryRegistry {
    async fn owner_of(&self, demos: u64) -> Result<Option<Ownership>, RegistryError> {
        let s = self.state.lock().unwrap();
        Ok(s.owners.get(&demos).map(|&(owner, epoch)| Ownership {
            demos,
            owner,
            epoch,
        }))
    }

    async fn claim(&self, demos: u64, node: NodeId) -> Result<ClaimOutcome, RegistryError> {
        let mut s = self.state.lock().unwrap();
        if let Some(&(by, epoch)) = s.owners.get(&demos) {
            return Ok(ClaimOutcome::Held { by, epoch });
        }
        // Enforce the founder-signed binding: only the chosen home node or a
        // pre-authorized failover heir may take ownership. A community with no
        // binding — or one whose binding does not verify against its key (poisoned /
        // unverifiable) — is unconstrained (legacy / imported / pre-feature).
        if let Some(binding) = s.home_bindings.get(&demos) {
            let key = s
                .community_keys
                .get(&demos)
                .and_then(|h| crate::CommunityPublicKey::from_hex(demos, h).ok());
            if binding_is_authoritative(binding, key.as_ref()) && !binding.authorizes(node.0) {
                return Err(RegistryError(format!(
                    "node {} is not authorized by the home binding for demos {demos}",
                    node.0
                )));
            }
        }
        // Fence: always bump past the highest epoch this community ever held.
        let epoch = s.max_epoch.get(&demos).copied().unwrap_or(0) + 1;
        s.owners.insert(demos, (node, epoch));
        s.max_epoch.insert(demos, epoch);
        Ok(ClaimOutcome::Claimed { epoch })
    }

    async fn release(&self, demos: u64, node: NodeId) -> Result<(), RegistryError> {
        let mut s = self.state.lock().unwrap();
        if s.owners.get(&demos).map(|&(o, _)| o) == Some(node) {
            s.owners.remove(&demos);
        }
        Ok(())
    }

    async fn set_standby(&self, demos: u64, node: NodeId) -> Result<(), RegistryError> {
        let list = self
            .state
            .lock()
            .unwrap()
            .standbys
            .entry(demos)
            .or_default()
            .clone();
        if !list.contains(&node.0) {
            self.state
                .lock()
                .unwrap()
                .standbys
                .entry(demos)
                .or_default()
                .push(node.0);
        }
        Ok(())
    }

    async fn standbys(&self, demos: u64) -> Result<Vec<NodeId>, RegistryError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .standbys
            .get(&demos)
            .map(|v| v.iter().map(|&n| NodeId(n)).collect())
            .unwrap_or_default())
    }

    async fn renew(&self, _node: NodeId) -> Result<(), RegistryError> {
        Ok(()) // no lease expiry in the in-memory model
    }

    async fn publish_community_key(
        &self,
        demos: u64,
        public_hex: &str,
        _origin_proof_hex: &str,
    ) -> Result<(), RegistryError> {
        // Single-node: no untrusted peers, so the origin proof is not enforced here.
        let mut s = self.state.lock().unwrap();
        match s.community_keys.get(&demos) {
            Some(existing) if existing == public_hex => Ok(()),
            Some(_) => Err(RegistryError(
                "community key already published; refusing to overwrite (first-write-wins)".into(),
            )),
            None => {
                s.community_keys.insert(demos, public_hex.to_string());
                Ok(())
            }
        }
    }

    async fn community_key(
        &self,
        demos: u64,
    ) -> Result<Option<crate::CommunityPublicKey>, RegistryError> {
        let hex = self.state.lock().unwrap().community_keys.get(&demos).cloned();
        match hex {
            None => Ok(None),
            Some(h) => crate::CommunityPublicKey::from_hex(demos, &h)
                .map(Some)
                .map_err(|e| RegistryError(e.to_string())),
        }
    }

    async fn set_home_binding(&self, binding: &crate::HomeBinding) -> Result<(), RegistryError> {
        let mut s = self.state.lock().unwrap();
        // Verify the binding against the community key BEFORE storing it. An
        // unverifiable binding is not a founder statement, so refuse it here rather
        // than let it poison later authorization/claim decisions (FED-3).
        let key = s
            .community_keys
            .get(&binding.demos)
            .and_then(|h| crate::CommunityPublicKey::from_hex(binding.demos, h).ok());
        if !binding_is_authoritative(binding, key.as_ref()) {
            return Err(RegistryError(
                "refusing to store a home binding that does not verify against the community key"
                    .into(),
            ));
        }
        if let Some(existing) = s.home_bindings.get(&binding.demos) {
            if binding.epoch < existing.epoch {
                return Err(RegistryError(
                    "refusing to install a lower-epoch home binding".into(),
                ));
            }
        }
        s.home_bindings.insert(binding.demos, binding.clone());
        Ok(())
    }

    async fn home_binding(
        &self,
        demos: u64,
    ) -> Result<Option<crate::HomeBinding>, RegistryError> {
        Ok(self.state.lock().unwrap().home_bindings.get(&demos).cloned())
    }

    async fn publish_key(&self, node: NodeId, public_hex: &str) -> Result<(), RegistryError> {
        // First-write-wins: a node's signing key is the anchor every
        // authorization decision trusts, so once published it must not be
        // silently overwritten with a different key. Re-publishing the *same*
        // key (e.g. on restart) is idempotent.
        let mut s = self.state.lock().unwrap();
        if let Some(existing) = s.keys.get(&node.0) {
            return if existing == public_hex {
                Ok(())
            } else {
                Err(RegistryError(
                    "node key already published; refusing to overwrite it (first-write-wins)"
                        .into(),
                ))
            };
        }
        s.keys.insert(node.0, public_hex.to_string());
        Ok(())
    }

    async fn public_key(&self, node: NodeId) -> Result<Option<NodePublicKey>, RegistryError> {
        let hex = self.state.lock().unwrap().keys.get(&node.0).cloned();
        match hex {
            None => Ok(None),
            Some(h) => NodePublicKey::from_hex(node, &h)
                .map(Some)
                .map_err(|e| RegistryError(e.to_string())),
        }
    }

    async fn report_load(&self, node: NodeId, load: NodeLoad) -> Result<(), RegistryError> {
        self.state.lock().unwrap().loads.insert(node.0, load);
        Ok(())
    }

    async fn live_nodes(&self) -> Result<Vec<NodeStatus>, RegistryError> {
        let s = self.state.lock().unwrap();
        Ok(s.loads
            .iter()
            .map(|(&n, &load)| NodeStatus {
                node: NodeId(n),
                load,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommunityKeypair, NodeKeypair};

    async fn registry_with_owner(kp: &NodeKeypair, demos: u64) -> (InMemoryRegistry, u64) {
        let reg = InMemoryRegistry::new();
        reg.publish_key(kp.node(), &kp.public().to_hex())
            .await
            .unwrap();
        let ClaimOutcome::Claimed { epoch } = reg.claim(demos, kp.node()).await.unwrap() else {
            panic!("first claim must succeed");
        };
        (reg, epoch)
    }

    #[tokio::test]
    async fn claim_is_refused_for_a_node_the_home_binding_does_not_authorize() {
        let reg = InMemoryRegistry::new();
        let demos = 7u64;
        let community = CommunityKeypair::generate(demos);
        reg.publish_community_key(demos, &community.public().to_hex(), "")
            .await
            .unwrap();
        // Founder pins home to node 1, with node 3 as a pre-authorized failover heir.
        reg.set_home_binding(&community.bind(1, vec![3], 1))
            .await
            .unwrap();

        // An outsider (node 2) cannot seize the unowned community.
        assert!(reg.claim(demos, NodeId(2)).await.is_err());
        // The chosen home may claim it.
        assert!(matches!(
            reg.claim(demos, NodeId(1)).await.unwrap(),
            ClaimOutcome::Claimed { .. }
        ));
        // On the home's downtime, a PRE-AUTHORIZED failover node may take over.
        reg.release(demos, NodeId(1)).await.unwrap();
        assert!(matches!(
            reg.claim(demos, NodeId(3)).await.unwrap(),
            ClaimOutcome::Claimed { .. }
        ));
    }

    #[tokio::test]
    async fn set_home_binding_rejects_a_binding_that_does_not_verify() {
        // FED-3: a binding that does not verify against the community key is refused
        // at write time, so it can never poison later authorization/claim decisions.
        let demos = 7u64;
        let reg = InMemoryRegistry::new();
        let community = CommunityKeypair::generate(demos);
        reg.publish_community_key(demos, &community.public().to_hex(), "")
            .await
            .unwrap();
        // Signed by a DIFFERENT community key → poison → refused.
        let attacker = CommunityKeypair::generate(demos);
        assert!(reg.set_home_binding(&attacker.bind(2, vec![], 9_999)).await.is_err());
        // The genuine binding is accepted.
        assert!(reg.set_home_binding(&community.bind(1, vec![], 1)).await.is_ok());
    }

    #[tokio::test]
    async fn a_second_claim_while_owned_does_not_take_ownership() {
        let a = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&a, 7).await;
        let outcome = reg.claim(7, NodeId(2)).await.unwrap();
        assert_eq!(
            outcome,
            ClaimOutcome::Held {
                by: a.node(),
                epoch
            }
        );
    }
}
