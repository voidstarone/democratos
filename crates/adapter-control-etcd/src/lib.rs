//! etcd-backed [`OwnershipRegistry`]: leases, epoch fencing, key & load registry.
//!
//! # Key layout
//!
//! ```text
//! democratos/keys/<node>            → public key hex        (persistent)
//! democratos/nodes/<node>           → NodeLoad JSON         (lease → liveness)
//! democratos/owners/<demos>/holder  → owner node id         (lease → ownership)
//! democratos/owners/<demos>/epoch   → monotonic epoch       (persistent)
//! ```
//!
//! The **holder** key is written with this node's lease, so if the node stops
//! heartbeating (crash / partition) etcd deletes it and the community becomes
//! claimable. The **epoch** key is persistent so it survives owner death — that is
//! what makes the fence monotonic: every [`claim`](OwnershipRegistry::claim) bumps
//! it, so a returning old owner always holds a strictly smaller epoch and is
//! rejected by [`federation::authorize`].
//!
//! # Claiming is a compare-and-swap
//!
//! etcd transactions cannot do read-modify-write of a value, so a claim is an
//! optimistic loop: read the epoch and its `mod_revision`, then a txn guarded on
//! (holder still absent) AND (epoch unchanged) that writes the holder (leased) and
//! the bumped epoch together. Two nodes racing to claim a lapsed community — the
//! rehoming case — cannot both win: exactly one txn's guard holds.

use std::time::Duration;

use async_trait::async_trait;
use etcd_client::{
    Certificate, Client, Compare, CompareOp, ConnectOptions, GetOptions, Identity, PutOptions,
    TlsOptions, Txn, TxnOp,
};

use domain::NodeId;
use federation::{ClaimOutcome, NodeLoad, NodeStatus, Ownership, OwnershipRegistry, RegistryError};

const PREFIX: &str = "democratos";

fn err(e: impl std::fmt::Display) -> RegistryError {
    RegistryError(e.to_string())
}

/// Build etcd connection TLS options from the environment, or `None` for a
/// plaintext connection (dev / an isolated trusted fabric).
///
/// * `DEMOCRATOS_ETCD_CA` — path to the CA cert (PEM). Setting it enables TLS: the
///   server's certificate is verified against this CA.
/// * `DEMOCRATOS_ETCD_CERT` + `DEMOCRATOS_ETCD_KEY` — this node's client cert & key
///   (PEM) for **mutual** TLS, so etcd can authenticate the node. Combine with
///   per-node etcd RBAC (each node scoped to its own keyspace) to make ownership
///   writes unforgeable — see deploy/README.md.
/// * `DEMOCRATOS_ETCD_DOMAIN` — optional server name override for verification.
fn etcd_tls_options() -> Result<Option<ConnectOptions>, RegistryError> {
    let Ok(ca_path) = std::env::var("DEMOCRATOS_ETCD_CA") else {
        return Ok(None);
    };
    let ca = std::fs::read(&ca_path)
        .map_err(|e| RegistryError(format!("read etcd CA {ca_path}: {e}")))?;
    let mut tls = TlsOptions::new().ca_certificate(Certificate::from_pem(ca));
    if let Ok(domain) = std::env::var("DEMOCRATOS_ETCD_DOMAIN") {
        tls = tls.domain_name(domain);
    }
    match (
        std::env::var("DEMOCRATOS_ETCD_CERT").ok(),
        std::env::var("DEMOCRATOS_ETCD_KEY").ok(),
    ) {
        (Some(cert_path), Some(key_path)) => {
            let cert = std::fs::read(&cert_path)
                .map_err(|e| RegistryError(format!("read etcd client cert {cert_path}: {e}")))?;
            let key = std::fs::read(&key_path)
                .map_err(|e| RegistryError(format!("read etcd client key {key_path}: {e}")))?;
            tls = tls.identity(Identity::from_pem(cert, key));
        }
        (None, None) => {}
        _ => {
            return Err(RegistryError(
                "etcd mutual TLS needs BOTH DEMOCRATOS_ETCD_CERT and DEMOCRATOS_ETCD_KEY".into(),
            ))
        }
    }
    Ok(Some(ConnectOptions::new().with_tls(tls)))
}

fn key_of(node: NodeId) -> String {
    format!("{PREFIX}/keys/{}", node.0)
}
fn node_of(node: NodeId) -> String {
    format!("{PREFIX}/nodes/{}", node.0)
}
fn holder_of(demos: u64) -> String {
    format!("{PREFIX}/owners/{demos}/holder")
}
fn epoch_of(demos: u64) -> String {
    format!("{PREFIX}/owners/{demos}/epoch")
}
fn standbys_of(demos: u64) -> String {
    format!("{PREFIX}/owners/{demos}/standbys")
}
fn community_key_of(demos: u64) -> String {
    format!("{PREFIX}/community/{demos}/key")
}
fn home_binding_of(demos: u64) -> String {
    format!("{PREFIX}/community/{demos}/binding")
}

/// An etcd-backed control plane bound to one node's lease.
pub struct EtcdRegistry {
    client: Client,
    /// This node's own identity — used to authorize control-plane writes that only
    /// the owner of a community may make (e.g. designating its standbys).
    node: NodeId,
    /// This node's lease; holder/liveness keys are written under it, and a
    /// background task keeps it alive.
    lease_id: i64,
    lease_ttl: i64,
}

impl EtcdRegistry {
    /// Connect to an etcd cluster as `node`, granting this node a lease with
    /// `ttl_secs`. A background task renews the lease every `ttl_secs/3`; if this
    /// process dies, the lease lapses within the TTL and everything it owns
    /// becomes claimable.
    pub async fn connect(
        endpoints: &[String],
        ttl_secs: i64,
        node: NodeId,
    ) -> Result<Self, RegistryError> {
        // TLS (and optional mutual TLS) for the control-plane link when configured.
        // The control plane is the ownership trust anchor: without TLS anyone who
        // can reach etcd can forge holder/epoch/key writes and seize any community.
        let opts = etcd_tls_options()?;
        let mut client = Client::connect(endpoints, opts).await.map_err(err)?;
        let lease = client.lease_grant(ttl_secs, None).await.map_err(err)?;
        let lease_id = lease.id();

        // Keep the lease alive for the life of the process.
        let (mut keeper, mut stream) = client.lease_keep_alive(lease_id).await.map_err(err)?;
        let period = Duration::from_secs((ttl_secs / 3).max(1) as u64);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(period).await;
                if keeper.keep_alive().await.is_err() {
                    break;
                }
                // Drain one ack so the stream doesn't back up.
                let _ = stream.message().await;
            }
        });

        Ok(Self {
            client,
            node,
            lease_id,
            lease_ttl: ttl_secs,
        })
    }

    /// The lease TTL this node registered with (seconds).
    pub fn lease_ttl(&self) -> i64 {
        self.lease_ttl
    }

    async fn get_str(&self, key: &str) -> Result<Option<(String, i64)>, RegistryError> {
        let mut client = self.client.clone();
        let resp = client.get(key, None).await.map_err(err)?;
        match resp.kvs().first() {
            None => Ok(None),
            Some(kv) => {
                let s = kv.value_str().map_err(err)?.to_string();
                Ok(Some((s, kv.mod_revision())))
            }
        }
    }
}

#[async_trait]
impl OwnershipRegistry for EtcdRegistry {
    async fn owner_of(&self, demos: u64) -> Result<Option<Ownership>, RegistryError> {
        // A holder key exists only while the owner's lease is alive.
        let Some((holder, _)) = self.get_str(&holder_of(demos)).await? else {
            return Ok(None);
        };
        let owner: u16 = holder.parse().map_err(err)?;
        let epoch = match self.get_str(&epoch_of(demos)).await? {
            Some((e, _)) => e.parse().map_err(err)?,
            None => 0,
        };
        Ok(Some(Ownership {
            demos,
            owner: NodeId(owner),
            epoch,
        }))
    }

    async fn claim(&self, demos: u64, node: NodeId) -> Result<ClaimOutcome, RegistryError> {
        // Enforce the founder-signed home binding: only the chosen home node or a
        // pre-authorized failover heir may take ownership. A community with no
        // binding is unconstrained (legacy / imported / pre-feature). This mirrors
        // the fleet-wide `authorize` check, so a hostile node can't even acquire the
        // lease for a community it isn't bound to.
        if let Some(binding) = self.home_binding(demos).await? {
            let community_key = self.community_key(demos).await?;
            // Only an authoritative binding (verifies against the community key)
            // constrains who may claim; a poisoned/unverifiable one is treated as
            // absent so it can neither grant a claim to an impostor nor block the
            // honest owner (FED-3).
            if federation::binding_is_authoritative(&binding, community_key.as_ref())
                && !binding.authorizes(node.0)
            {
                return Err(RegistryError(format!(
                    "node {} is not authorized by the home binding for demos {demos}",
                    node.0
                )));
            }
        }
        let holder_key = holder_of(demos);
        let epoch_key = epoch_of(demos);
        let mut client = self.client.clone();

        // Optimistic CAS loop. Bounded so a pathological livelock can't spin forever.
        for _ in 0..8 {
            if let Some((holder, _)) = self.get_str(&holder_key).await? {
                let by: u16 = holder.parse().map_err(err)?;
                let epoch = self
                    .get_str(&epoch_key)
                    .await?
                    .map(|(e, _)| e.parse().unwrap_or(0))
                    .unwrap_or(0);
                return Ok(ClaimOutcome::Held {
                    by: NodeId(by),
                    epoch,
                });
            }

            let (cur_epoch, epoch_rev) = match self.get_str(&epoch_key).await? {
                Some((e, rev)) => (e.parse::<u64>().map_err(err)?, rev),
                None => (0, 0), // absent → create_revision/mod_revision guard is 0
            };
            let next_epoch = cur_epoch + 1;

            let txn = Txn::new()
                .when(vec![
                    // Holder still absent (nobody claimed since we looked).
                    Compare::create_revision(holder_key.as_bytes(), CompareOp::Equal, 0),
                    // Epoch unchanged since we read it.
                    Compare::mod_revision(epoch_key.as_bytes(), CompareOp::Equal, epoch_rev),
                ])
                .and_then(vec![
                    TxnOp::put(
                        holder_key.as_bytes(),
                        node.0.to_string(),
                        Some(PutOptions::new().with_lease(self.lease_id)),
                    ),
                    TxnOp::put(epoch_key.as_bytes(), next_epoch.to_string(), None),
                ]);

            if client.txn(txn).await.map_err(err)?.succeeded() {
                return Ok(ClaimOutcome::Claimed { epoch: next_epoch });
            }
            // Lost the race; re-read and retry.
        }
        Err(RegistryError("claim contended out after retries".into()))
    }

    async fn release(&self, demos: u64, node: NodeId) -> Result<(), RegistryError> {
        let holder_key = holder_of(demos);
        let mut client = self.client.clone();
        // Only delete if we still hold it (guard on the value being our node id).
        let txn = Txn::new()
            .when(vec![Compare::value(
                holder_key.as_bytes(),
                CompareOp::Equal,
                node.0.to_string(),
            )])
            .and_then(vec![TxnOp::delete(holder_key.as_bytes(), None)]);
        client.txn(txn).await.map_err(err)?;
        Ok(())
    }

    async fn set_standby(&self, demos: u64, node: NodeId) -> Result<(), RegistryError> {
        // Only the community's CURRENT OWNER may designate its standbys. A standby
        // is a failover heir (the rehoming controller promotes one when the owner
        // goes down), so letting any node append *itself* would let a hostile node
        // make itself the heir and seize the community on the owner's next downtime.
        // The put is guarded on "I am the current holder", enforced by etcd — a
        // non-owner's write simply does not apply.
        let holder_key = holder_of(demos);
        let key = standbys_of(demos);
        // Persistent list (survives owner death — a standby is a failover target).
        let mut list: Vec<u16> = match self.get_str(&key).await? {
            Some((json, _)) => serde_json::from_str(&json).map_err(err)?,
            None => Vec::new(),
        };
        if list.contains(&node.0) {
            return Ok(());
        }
        list.push(node.0);
        let json = serde_json::to_string(&list).map_err(err)?;
        let mut client = self.client.clone();
        let txn = Txn::new()
            .when(vec![Compare::value(
                holder_key.as_bytes(),
                CompareOp::Equal,
                self.node.0.to_string(),
            )])
            .and_then(vec![TxnOp::put(key.as_bytes(), json, None)]);
        if !client.txn(txn).await.map_err(err)?.succeeded() {
            return Err(RegistryError(format!(
                "refusing to set a standby for demos {demos}: this node is not its current owner"
            )));
        }
        Ok(())
    }

    async fn standbys(&self, demos: u64) -> Result<Vec<NodeId>, RegistryError> {
        match self.get_str(&standbys_of(demos)).await? {
            None => Ok(Vec::new()),
            Some((json, _)) => {
                let list: Vec<u16> = serde_json::from_str(&json).map_err(err)?;
                Ok(list.into_iter().map(NodeId).collect())
            }
        }
    }

    async fn renew(&self, _node: NodeId) -> Result<(), RegistryError> {
        // The background keep-alive task renews the lease; nothing to do here.
        Ok(())
    }

    async fn publish_community_key(
        &self,
        demos: u64,
        public_hex: &str,
        origin_proof_hex: &str,
    ) -> Result<(), RegistryError> {
        // Origin authentication (FED-1): the community key must be published with a
        // signature by the community's ORIGIN node (its id's high bits) over the
        // canonical challenge, verified against that node's published key. This ties
        // the community key to the honest founding node, so a hostile peer cannot
        // pre-empt or hijack it via a race on the etcd key. A community whose origin
        // node has no published key cannot be secured this way — it stays unbound
        // (permissive), exactly as a legacy/imported community does today.
        let origin = domain::origin_node(demos);
        let origin_key = self
            .public_key(origin)
            .await?
            .ok_or_else(|| RegistryError(format!(
                "cannot publish community key for demos {demos}: its origin node {} has no published key",
                origin.0
            )))?;
        let challenge = federation::community_key_publish_challenge(demos, public_hex);
        origin_key
            .verify_hex(challenge.as_bytes(), origin_proof_hex)
            .map_err(|e| RegistryError(format!(
                "community key publish not authorised by origin node {}: {e}",
                origin.0
            )))?;

        // First-write-wins, exactly like a node key: a community's public key is
        // the anchor its home binding is verified against, so it must not be
        // overwritten once set. Re-publishing the same key is idempotent.
        let key = community_key_of(demos);
        if let Some((existing, _)) = self.get_str(&key).await? {
            return if existing == public_hex {
                Ok(())
            } else {
                Err(RegistryError(
                    "community key already published; refusing to overwrite (first-write-wins)"
                        .into(),
                ))
            };
        }
        let mut client = self.client.clone();
        let txn = Txn::new()
            .when(vec![Compare::create_revision(
                key.as_bytes(),
                CompareOp::Equal,
                0,
            )])
            .and_then(vec![TxnOp::put(key.as_bytes(), public_hex, None)]);
        if !client.txn(txn).await.map_err(err)?.succeeded() {
            return Err(RegistryError(
                "community key was published concurrently; refusing to overwrite".into(),
            ));
        }
        Ok(())
    }

    async fn community_key(
        &self,
        demos: u64,
    ) -> Result<Option<federation::CommunityPublicKey>, RegistryError> {
        match self.get_str(&community_key_of(demos)).await? {
            None => Ok(None),
            Some((hex, _)) => federation::CommunityPublicKey::from_hex(demos, &hex)
                .map(Some)
                .map_err(err),
        }
    }

    async fn set_home_binding(
        &self,
        binding: &federation::HomeBinding,
    ) -> Result<(), RegistryError> {
        // Verify the binding against the community key BEFORE storing (FED-3): an
        // unverifiable binding is not a founder statement, so refuse it here rather
        // than let a party with control-plane write access poison later
        // authorization/claim decisions (which would DoS the honest owner).
        let community_key = self.community_key(binding.demos).await?;
        if !federation::binding_is_authoritative(binding, community_key.as_ref()) {
            return Err(RegistryError(
                "refusing to store a home binding that does not verify against the community key"
                    .into(),
            ));
        }
        // Keep the highest-epoch binding (a re-home bumps it); refuse a downgrade.
        let key = home_binding_of(binding.demos);
        if let Some((json, _)) = self.get_str(&key).await? {
            if let Ok(existing) = serde_json::from_str::<federation::HomeBinding>(&json) {
                if binding.epoch < existing.epoch {
                    return Err(RegistryError(
                        "refusing to install a lower-epoch home binding".into(),
                    ));
                }
            }
        }
        let json = serde_json::to_string(binding).map_err(err)?;
        let mut client = self.client.clone();
        client.put(key, json, None).await.map_err(err)?;
        Ok(())
    }

    async fn home_binding(
        &self,
        demos: u64,
    ) -> Result<Option<federation::HomeBinding>, RegistryError> {
        match self.get_str(&home_binding_of(demos)).await? {
            None => Ok(None),
            Some((json, _)) => serde_json::from_str(&json).map(Some).map_err(err),
        }
    }

    async fn publish_key(&self, node: NodeId, public_hex: &str) -> Result<(), RegistryError> {
        // Persistent (no lease): peers must be able to verify a node's past events
        // even while it is down.
        //
        // First-write-wins: a node's signing key is the identity anchor every
        // authorization decision trusts (`authorize`, `verify_signed`). Once
        // published, refuse to overwrite it with a *different* key, so a party
        // with mere etcd write access can't hijack a live node's identity.
        // Re-publishing the same key (e.g. on restart) is idempotent; a genuine
        // key rotation is a deliberate, out-of-band operation.
        //
        // NOTE: this is defense-in-depth. Full protection still requires per-node
        // authentication on this write (a signed key-publish), so a first-ever
        // publish for a not-yet-seen node cannot be pre-empted by an attacker —
        // see the security review's federation trust-anchor findings.
        let key = key_of(node);
        if let Some((existing, _)) = self.get_str(&key).await? {
            return if existing == public_hex {
                Ok(())
            } else {
                Err(RegistryError(
                    "node key already published; refusing to overwrite it (first-write-wins)"
                        .into(),
                ))
            };
        }
        // Absent → create only if still absent, so a racing writer can't slip in.
        let mut client = self.client.clone();
        let txn = Txn::new()
            .when(vec![Compare::create_revision(
                key.as_bytes(),
                CompareOp::Equal,
                0,
            )])
            .and_then(vec![TxnOp::put(key.as_bytes(), public_hex, None)]);
        if !client.txn(txn).await.map_err(err)?.succeeded() {
            return Err(RegistryError(
                "node key was published concurrently; refusing to overwrite it".into(),
            ));
        }
        Ok(())
    }

    async fn public_key(
        &self,
        node: NodeId,
    ) -> Result<Option<federation::NodePublicKey>, RegistryError> {
        match self.get_str(&key_of(node)).await? {
            None => Ok(None),
            Some((hex, _)) => federation::NodePublicKey::from_hex(node, &hex)
                .map(Some)
                .map_err(err),
        }
    }

    async fn report_load(&self, node: NodeId, load: NodeLoad) -> Result<(), RegistryError> {
        // Leased: presence under nodes/ is this node's liveness signal.
        let json = serde_json::to_string(&LoadWire::from(load)).map_err(err)?;
        let mut client = self.client.clone();
        client
            .put(
                node_of(node),
                json,
                Some(PutOptions::new().with_lease(self.lease_id)),
            )
            .await
            .map_err(err)?;
        Ok(())
    }

    async fn live_nodes(&self) -> Result<Vec<NodeStatus>, RegistryError> {
        let mut client = self.client.clone();
        let resp = client
            .get(
                format!("{PREFIX}/nodes/"),
                Some(GetOptions::new().with_prefix()),
            )
            .await
            .map_err(err)?;
        let mut out = Vec::new();
        for kv in resp.kvs() {
            let key = kv.key_str().map_err(err)?;
            let Some(id) = key.rsplit('/').next().and_then(|s| s.parse::<u16>().ok()) else {
                continue;
            };
            let load: LoadWire = serde_json::from_slice(kv.value()).map_err(err)?;
            out.push(NodeStatus {
                node: NodeId(id),
                load: load.into(),
            });
        }
        Ok(out)
    }
}

/// Wire form of `NodeLoad` (the federation type isn't `Serialize`, keeping that
/// crate serde-light; the mapping lives here where etcd needs it).
#[derive(serde::Serialize, serde::Deserialize)]
struct LoadWire {
    hosted_communities: u32,
    requests_per_sec: f64,
}
impl From<NodeLoad> for LoadWire {
    fn from(l: NodeLoad) -> Self {
        Self {
            hosted_communities: l.hosted_communities,
            requests_per_sec: l.requests_per_sec,
        }
    }
}
impl From<LoadWire> for NodeLoad {
    fn from(w: LoadWire) -> Self {
        NodeLoad {
            hosted_communities: w.hosted_communities,
            requests_per_sec: w.requests_per_sec,
        }
    }
}
