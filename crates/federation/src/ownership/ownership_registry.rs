//! The control plane trait.

use async_trait::async_trait;

use domain::NodeId;

use crate::{ClaimOutcome, NodeLoad, NodePublicKey, NodeStatus, Ownership, RegistryError};

/// The control plane. etcd implements this for production (leases, epoch fencing
/// via compare-and-swap, key distribution, load reporting);
/// [`InMemoryRegistry`](crate::InMemoryRegistry) implements it for a single node
/// and for tests.
#[async_trait]
pub trait OwnershipRegistry: Send + Sync {
    /// Current owner + epoch of a community, or `None` if unowned (never claimed,
    /// or the owner's lease has lapsed).
    async fn owner_of(&self, demos: u64) -> Result<Option<Ownership>, RegistryError>;

    /// Attempt to take ownership of `demos` for `node`. Succeeds only if the
    /// community is currently unowned; on success the epoch is **bumped** (fencing
    /// any prior owner). If a live node already holds it, returns [`ClaimOutcome::Held`].
    async fn claim(&self, demos: u64, node: NodeId) -> Result<ClaimOutcome, RegistryError>;

    /// Gracefully give up ownership of `demos` (planned handoff). No-op if `node`
    /// is not the current owner.
    async fn release(&self, demos: u64, node: NodeId) -> Result<(), RegistryError>;

    /// Designate `node` as a **standby** (synchronous replica) for `demos`. A
    /// vote's owner replicates to a standby before acking (quorum of 2), and a
    /// standby is the pre-warmed, caught-up target for failover.
    async fn set_standby(&self, demos: u64, node: NodeId) -> Result<(), RegistryError>;

    /// The standbys currently designated for `demos`.
    async fn standbys(&self, demos: u64) -> Result<Vec<NodeId>, RegistryError>;

    /// Heartbeat: renew this node's lease so the communities it owns stay owned.
    async fn renew(&self, node: NodeId) -> Result<(), RegistryError>;

    /// Publish this node's public key (hex) so peers can verify its events.
    async fn publish_key(&self, node: NodeId, public_hex: &str) -> Result<(), RegistryError>;

    /// Fetch a node's published public key, if any.
    async fn public_key(&self, node: NodeId) -> Result<Option<NodePublicKey>, RegistryError>;

    /// Report this node's current load, for placement decisions.
    async fn report_load(&self, node: NodeId, load: NodeLoad) -> Result<(), RegistryError>;

    /// All currently-live nodes with their last-reported load.
    async fn live_nodes(&self) -> Result<Vec<NodeStatus>, RegistryError>;

    // --- founder-signed home bindings (open-federation ownership anchor) -----
    //
    // These default to a permissive no-op so a registry that predates the feature
    // (or a community founded before it) behaves exactly as before — ownership is
    // unconstrained. Once a community publishes a key and a binding, `authorize`
    // and `claim` enforce it. See [`crate::binding`].

    /// Publish a community's public key. First-write-wins: a community's key is the
    /// anchor its home binding is verified against, so once set it is not silently
    /// overwritten. Re-publishing the same key is idempotent.
    ///
    /// `origin_proof_hex` is an Ed25519 signature by the community's **origin node**
    /// (`domain::origin_node(demos)`) over [`crate::community_key_publish_challenge`].
    /// A federated registry verifies it against the origin node's published key, so a
    /// hostile peer cannot pre-empt or hijack the key of a community founded by an
    /// honest node (FED-1). A single-node registry ([`InMemoryRegistry`](crate::InMemoryRegistry))
    /// has no untrusted peers and ignores it.
    async fn publish_community_key(
        &self,
        _demos: u64,
        _public_hex: &str,
        _origin_proof_hex: &str,
    ) -> Result<(), RegistryError> {
        Ok(())
    }

    /// A community's published public key, if any.
    async fn community_key(
        &self,
        _demos: u64,
    ) -> Result<Option<crate::CommunityPublicKey>, RegistryError> {
        Ok(None)
    }

    /// Store a community's current home binding. The caller must have verified it
    /// against the community key; implementations keep the highest-epoch binding.
    async fn set_home_binding(
        &self,
        _binding: &crate::HomeBinding,
    ) -> Result<(), RegistryError> {
        Ok(())
    }

    /// A community's current (highest-epoch) home binding, if any.
    async fn home_binding(
        &self,
        _demos: u64,
    ) -> Result<Option<crate::HomeBinding>, RegistryError> {
        Ok(None)
    }

    // --- federation-root-signed trusted issuers (global-account anchor) --------
    //
    // User accounts are global rows with no per-community owner. These gate which
    // nodes may mint accounts that replicate fleet-wide. They default permissively
    // (every node trusted) so a registry with no configured trust root behaves
    // exactly as before — enforcement turns on only once a root is configured and
    // certs are published. See [`crate::issuer`].

    /// Whether `node` is a federation-trusted account issuer — i.e. holds a valid
    /// root-signed [`IssuerCert`](crate::IssuerCert). Consulted by
    /// [`authorize`](crate::authorize) on a global (user-account) event. Defaults
    /// to `true`: a registry with no trust root configured trusts every node, as
    /// before. A registry configured with a root returns `true` only for a node
    /// whose stored cert verifies against it.
    async fn is_trusted_issuer(&self, _node: NodeId) -> Result<bool, RegistryError> {
        Ok(true)
    }

    /// Store a node's trusted-issuer certificate. The caller must have verified it
    /// against the federation root key; implementations keep the highest-epoch cert.
    async fn set_issuer_cert(&self, _cert: &crate::IssuerCert) -> Result<(), RegistryError> {
        Ok(())
    }

    /// A node's current (highest-epoch) trusted-issuer certificate, if any.
    async fn issuer_cert(&self, _node: NodeId) -> Result<Option<crate::IssuerCert>, RegistryError> {
        Ok(None)
    }

    /// Publish this node's base URL for the federation command endpoint, so a peer
    /// that needs to forward account minting/login can discover where to reach it.
    /// `sig` is the node's own Ed25519 signature (hex) over
    /// [`node_addr_challenge`](crate::node_addr_challenge)`(node, url)`; storing it
    /// lets [`node_addr`](Self::node_addr) reject an address the node did not sign,
    /// so a party with control-plane write access cannot redirect forwarded
    /// credentials. Defaults to a no-op for registries that carry no addresses.
    async fn publish_addr(
        &self,
        _node: NodeId,
        _url: &str,
        _sig: &str,
    ) -> Result<(), RegistryError> {
        Ok(())
    }

    /// A node's published command base URL — returned **only if** its stored
    /// signature verifies against the node's published key. An unsigned, badly-signed,
    /// or key-less address returns `None` (fail closed): forwarded credentials never
    /// go to an address the target node did not vouch for.
    async fn node_addr(&self, _node: NodeId) -> Result<Option<String>, RegistryError> {
        Ok(None)
    }

    /// Atomically reserve `handle` for `node` in the fleet-wide handle namespace, so
    /// two trusted issuers can't independently mint the same handle (which — because
    /// login resolves by handle — would let a colliding account impersonate another).
    /// Returns `true` **only for the request that freshly reserves it**; a handle that
    /// already exists (even one this node holds) returns `false`. This non-idempotence
    /// is deliberate: it makes [`release_handle`](Self::release_handle)-on-failure safe
    /// under concurrency (a duplicate mint gets `false` and never releases the winner's
    /// handle). Defaults to `true`: with no coordinating control plane there is a
    /// single issuer, so the local uniqueness check suffices (single-node/legacy).
    async fn reserve_handle(&self, _handle: &str, _node: NodeId) -> Result<bool, RegistryError> {
        Ok(true)
    }

    /// Release a handle reservation held by `node` (used to undo a reservation when
    /// the subsequent account creation fails, so a rejected sign-up doesn't strand the
    /// handle). A no-op if `node` does not hold it. Defaults to a no-op.
    async fn release_handle(&self, _handle: &str, _node: NodeId) -> Result<(), RegistryError> {
        Ok(())
    }

    /// All handles currently reserved by `node`. Used at startup to reconcile away
    /// **orphans** — a handle reserved when a crash interrupted the reserve→create
    /// window, so it has no account. Defaults to an empty list.
    async fn reserved_handles(&self, _node: NodeId) -> Result<Vec<String>, RegistryError> {
        Ok(Vec::new())
    }

    /// Discover the trusted account issuers a node can mint through: the live nodes
    /// that hold a valid issuer cert **and** have published a reachable address. The
    /// default composes the primitives above, so an adapter only implements storage
    /// ([`publish_addr`](Self::publish_addr)/[`node_addr`](Self::node_addr)) — never
    /// this policy. Selection among the result is [`crate::choose_issuer`].
    async fn trusted_issuers(&self) -> Result<Vec<crate::IssuerEndpoint>, RegistryError> {
        let mut out = Vec::new();
        for status in self.live_nodes().await? {
            if !self.is_trusted_issuer(status.node).await? {
                continue;
            }
            if let Some(addr) = self.node_addr(status.node).await? {
                out.push(crate::IssuerEndpoint {
                    node: status.node,
                    addr,
                    load: status.load,
                });
            }
        }
        Ok(out)
    }
}
