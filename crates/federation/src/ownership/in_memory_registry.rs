//! A process-local `OwnershipRegistry` for single-node / dev / tests.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;

use domain::NodeId;

use crate::{
    binding_is_authoritative, ClaimOutcome, IssuerCert, IssuerRootPublicKey, NodeLoad,
    NodePublicKey, NodeStatus, Ownership, OwnershipRegistry, RegistryError,
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
    /// node → its current (highest-epoch) root-signed trusted-issuer cert.
    issuer_certs: HashMap<u16, IssuerCert>,
    /// node → its published command base URL and the node's signature over it.
    addrs: HashMap<u16, (String, String)>,
    /// handle → the node that reserved it (fleet-wide handle namespace).
    handles: HashMap<String, u16>,
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
    /// The federation trust root, if this deployment enforces trusted issuers.
    /// `None` (the default) trusts every node's accounts — the legacy behaviour a
    /// single-node/dev registry wants. `Some` gates global accounts on a valid cert.
    trust_root: Option<IssuerRootPublicKey>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry that **enforces trusted issuers** against `root`: a global
    /// (user-account) event is authorized only if its minting node holds a cert
    /// verifiable against `root`.
    pub fn with_trust_root(root: IssuerRootPublicKey) -> Self {
        Self {
            state: Mutex::default(),
            trust_root: Some(root),
        }
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

    async fn is_trusted_issuer(&self, node: NodeId) -> Result<bool, RegistryError> {
        // No trust root configured → trust every node (single-node / dev / legacy).
        let Some(root) = self.trust_root.as_ref() else {
            return Ok(true);
        };
        let cert = self.state.lock().unwrap().issuer_certs.get(&node.0).cloned();
        Ok(match cert {
            Some(cert) => cert.verify(root).is_ok() && cert.certifies(node.0),
            None => false,
        })
    }

    async fn set_issuer_cert(&self, cert: &IssuerCert) -> Result<(), RegistryError> {
        // Verify against the configured root BEFORE storing, so an unverifiable cert
        // can never poison later authorization (mirrors `set_home_binding`). With no
        // root configured there is nothing to verify against — store it as-is (dev).
        if let Some(root) = self.trust_root.as_ref() {
            cert.verify(root)
                .map_err(|e| RegistryError(format!("issuer cert does not verify: {e}")))?;
        }
        let mut s = self.state.lock().unwrap();
        if let Some(existing) = s.issuer_certs.get(&cert.node) {
            if cert.epoch < existing.epoch {
                return Err(RegistryError(
                    "refusing to install a lower-epoch issuer cert".into(),
                ));
            }
        }
        s.issuer_certs.insert(cert.node, cert.clone());
        Ok(())
    }

    async fn issuer_cert(&self, node: NodeId) -> Result<Option<IssuerCert>, RegistryError> {
        Ok(self.state.lock().unwrap().issuer_certs.get(&node.0).cloned())
    }

    async fn reserve_handle(&self, handle: &str, node: NodeId) -> Result<bool, RegistryError> {
        // NON-idempotent: `true` only for the request that FRESHLY reserves it. A
        // handle that already exists — even one this node holds — returns `false`, so
        // exactly one in-flight request ever "owns" a fresh reservation. That is what
        // keeps release-on-failure safe: a concurrent duplicate mint gets `false` and
        // never reaches the release path, so it can't strand the winner's handle.
        let mut s = self.state.lock().unwrap();
        if s.handles.contains_key(handle) {
            return Ok(false);
        }
        s.handles.insert(handle.to_string(), node.0);
        Ok(true)
    }

    async fn release_handle(&self, handle: &str, node: NodeId) -> Result<(), RegistryError> {
        let mut s = self.state.lock().unwrap();
        if s.handles.get(handle) == Some(&node.0) {
            s.handles.remove(handle);
        }
        Ok(())
    }

    async fn reserved_handles(&self, node: NodeId) -> Result<Vec<String>, RegistryError> {
        let s = self.state.lock().unwrap();
        Ok(s.handles
            .iter()
            .filter(|(_, &owner)| owner == node.0)
            .map(|(h, _)| h.clone())
            .collect())
    }

    async fn publish_addr(&self, node: NodeId, url: &str, sig: &str) -> Result<(), RegistryError> {
        self.state
            .lock()
            .unwrap()
            .addrs
            .insert(node.0, (url.to_string(), sig.to_string()));
        Ok(())
    }

    async fn node_addr(&self, node: NodeId) -> Result<Option<String>, RegistryError> {
        let (stored, key_hex) = {
            let s = self.state.lock().unwrap();
            let Some((url, sig)) = s.addrs.get(&node.0) else {
                return Ok(None);
            };
            // A node's address is only trustworthy if the node itself signed it —
            // otherwise a control-plane writer could redirect forwarded credentials.
            let Some(key_hex) = s.keys.get(&node.0).cloned() else {
                return Ok(None);
            };
            ((url.clone(), sig.clone()), key_hex)
        };
        let (url, sig) = stored;
        let Ok(key) = NodePublicKey::from_hex(node, &key_hex) else {
            return Ok(None);
        };
        let challenge = crate::node_addr_challenge(node.0, &url);
        Ok(key.verify_hex(challenge.as_bytes(), &sig).ok().map(|_| url))
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
    use crate::{CommunityKeypair, IssuerRootKeypair, NodeKeypair};

    /// The node's own signature over its advertised address (what a real node
    /// publishes), so `node_addr` accepts it.
    fn addr_sig(kp: &NodeKeypair, url: &str) -> String {
        kp.sign_hex(crate::node_addr_challenge(kp.node().0, url).as_bytes())
    }

    #[tokio::test]
    async fn trusted_issuers_lists_only_live_certified_nodes_with_an_address() {
        let root = IssuerRootKeypair::generate();
        let reg = InMemoryRegistry::with_trust_root(root.public());

        // Node 1 — certified, live, addressed → discoverable.
        let n1 = NodeKeypair::generate(NodeId(1));
        reg.publish_key(n1.node(), &n1.public().to_hex()).await.unwrap();
        reg.set_issuer_cert(&root.certify(1, 1)).await.unwrap();
        reg.report_load(NodeId(1), NodeLoad::default()).await.unwrap();
        reg.publish_addr(NodeId(1), "https://issuer1", &addr_sig(&n1, "https://issuer1"))
            .await
            .unwrap();

        // Node 2 — live and addressed but NOT certified → excluded.
        reg.report_load(NodeId(2), NodeLoad::default()).await.unwrap();
        let n2 = NodeKeypair::generate(NodeId(2));
        reg.publish_key(n2.node(), &n2.public().to_hex()).await.unwrap();
        reg.publish_addr(NodeId(2), "https://node2", &addr_sig(&n2, "https://node2"))
            .await
            .unwrap();

        // Node 3 — certified and live but no published address → excluded (can't reach).
        let n3 = NodeKeypair::generate(NodeId(3));
        reg.publish_key(n3.node(), &n3.public().to_hex()).await.unwrap();
        reg.set_issuer_cert(&root.certify(3, 1)).await.unwrap();
        reg.report_load(NodeId(3), NodeLoad::default()).await.unwrap();

        let issuers = reg.trusted_issuers().await.unwrap();
        assert_eq!(issuers.len(), 1, "only the certified, addressed, live node");
        assert_eq!(issuers[0].node, NodeId(1));
        assert_eq!(issuers[0].addr, "https://issuer1");
    }

    #[tokio::test]
    async fn discovery_never_routes_to_a_forged_cert_or_offline_node() {
        // ADVERSARIAL: two ways a rogue node might try to get picked to mint on.
        let root = IssuerRootKeypair::generate();
        let wrong_root = IssuerRootKeypair::generate();
        let reg = InMemoryRegistry::with_trust_root(root.public());

        // (1) Rogue node 9: live + advertises an address, but its "cert" is signed by
        // a key that is NOT the federation root. The registry refuses it at write
        // time, so it never becomes discoverable.
        let n9 = NodeKeypair::generate(NodeId(9));
        reg.publish_key(n9.node(), &n9.public().to_hex()).await.unwrap();
        assert!(
            reg.set_issuer_cert(&wrong_root.certify(9, 1)).await.is_err(),
            "a cert not signed by the true root is refused at write time"
        );
        reg.report_load(NodeId(9), NodeLoad::default()).await.unwrap();
        reg.publish_addr(NodeId(9), "https://rogue", &addr_sig(&n9, "https://rogue"))
            .await
            .unwrap();

        // (2) Node 5: genuinely certified and addressed, but OFFLINE (no live lease /
        // load) — a dead issuer must not be handed mints.
        let n5 = NodeKeypair::generate(NodeId(5));
        reg.publish_key(n5.node(), &n5.public().to_hex()).await.unwrap();
        reg.set_issuer_cert(&root.certify(5, 1)).await.unwrap();
        reg.publish_addr(NodeId(5), "https://issuer5", &addr_sig(&n5, "https://issuer5"))
            .await
            .unwrap();

        let issuers = reg.trusted_issuers().await.unwrap();
        assert!(
            issuers.is_empty(),
            "neither a forged-cert node nor an offline one is a discoverable issuer"
        );
        // And the rogue node is not considered a trusted issuer even directly.
        assert!(!reg.is_trusted_issuer(NodeId(9)).await.unwrap());
    }

    #[tokio::test]
    async fn a_handle_can_be_reserved_by_only_one_issuer() {
        // ADVERSARIAL: two trusted issuers both try to mint handle "alice". The
        // fleet-wide reservation lets exactly one win, so login-by-handle can never
        // resolve to a colliding impostor account.
        let reg = InMemoryRegistry::new();
        assert!(reg.reserve_handle("alice", NodeId(1)).await.unwrap(), "first issuer wins");
        assert!(
            !reg.reserve_handle("alice", NodeId(2)).await.unwrap(),
            "a second issuer cannot claim the same handle"
        );
        // NON-idempotent: even the holder gets false on re-reserve, so a concurrent
        // duplicate mint on the same issuer can't clobber the winner's reservation.
        assert!(
            !reg.reserve_handle("alice", NodeId(1)).await.unwrap(),
            "a re-reserve (even by the holder) does not report fresh ownership"
        );
        // A different handle is free.
        assert!(reg.reserve_handle("bob", NodeId(2)).await.unwrap());
        // Reconcile listing: node 1 holds "alice", node 2 holds "bob".
        assert_eq!(reg.reserved_handles(NodeId(1)).await.unwrap(), vec!["alice".to_string()]);
        assert_eq!(reg.reserved_handles(NodeId(2)).await.unwrap(), vec!["bob".to_string()]);
        // Releasing is guarded by the holder id: a non-holder's release is a no-op.
        reg.release_handle("alice", NodeId(2)).await.unwrap(); // not the holder: no-op
        assert!(!reg.reserve_handle("alice", NodeId(2)).await.unwrap(), "still held by node 1");
        reg.release_handle("alice", NodeId(1)).await.unwrap();
        assert!(reg.reserve_handle("alice", NodeId(2)).await.unwrap(), "now free to claim");
    }

    #[tokio::test]
    async fn a_poisoned_address_signed_by_the_wrong_key_is_not_handed_out() {
        // ADVERSARIAL: a party with control-plane write access tries to redirect
        // node 1's forwarded credentials to a server it controls, by writing an
        // address it signed with ITS OWN key. `node_addr` verifies the signature
        // against node 1's published key, so the poisoned address is invisible —
        // credentials never get forwarded to the attacker.
        let reg = InMemoryRegistry::new();
        let victim = NodeKeypair::generate(NodeId(1));
        reg.publish_key(victim.node(), &victim.public().to_hex()).await.unwrap();

        let attacker = NodeKeypair::generate(NodeId(1)); // same claimed id, different key
        let evil_url = "https://attacker.example";
        // Signed by the attacker's key, not node 1's real key.
        let evil_sig = attacker.sign_hex(crate::node_addr_challenge(1, evil_url).as_bytes());
        reg.publish_addr(NodeId(1), evil_url, &evil_sig).await.unwrap();
        assert_eq!(
            reg.node_addr(NodeId(1)).await.unwrap(),
            None,
            "an address the node did not sign must not be returned"
        );

        // The node's own genuine signature IS accepted.
        let good_url = "https://node1.example";
        let good_sig = victim.sign_hex(crate::node_addr_challenge(1, good_url).as_bytes());
        reg.publish_addr(NodeId(1), good_url, &good_sig).await.unwrap();
        assert_eq!(
            reg.node_addr(NodeId(1)).await.unwrap().as_deref(),
            Some(good_url)
        );
    }

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
