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
}
