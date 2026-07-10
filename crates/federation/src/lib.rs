//! Federation core: node identity and the signed change-event envelope.
//!
//! # Threat model
//!
//! Once communities replicate across a network, the transport and the peers are
//! **untrusted**. A hostile or compromised node must not be able to:
//!
//! * **forge** an event (invent a vote, a membership, a removal) that a peer
//!   would apply to its replica;
//! * **tamper** with a legitimate event in flight;
//! * **replay** an old event, or replay one community's event against another;
//! * **impersonate** a community's owner — which is what would let an attacker
//!   split the electorate and defeat the anti-takeover design.
//!
//! # The control
//!
//! Every event that leaves a node is **Ed25519-signed** by that node over a
//! canonical encoding of its *signed part* — `(node, epoch, seq, demos, entity,
//! op, payload)`. A consumer verifies the signature against the producer's public
//! key before applying anything. Binding `epoch` and `seq` into the signature
//! gives replay and split-brain protection: an event minted under a stale
//! ownership epoch, or below the consumer's cursor, is rejected even if correctly
//! signed. Binding `demos` stops an event being replayed against another
//! community. The key that must match is *the rightful owner's* — resolved from
//! the control plane (Phase 4); this crate provides the verification primitive.
//!
//! Node keys are **persistent identities**: a node loads its 32-byte seed from a
//! keyfile or secret env var, never generates one per boot (that would change its
//! identity and invalidate everything it ever signed). [`NodeKeypair::generate`]
//! exists only for keygen tooling and tests.

pub mod binding;
pub mod change_event;
pub mod change_op;
pub mod fed_error;
mod from_hex;
pub mod node_keypair;
pub mod node_public_key;
pub mod ownership;
pub mod rehome;
pub mod signed_part;
mod to_hex;

pub use change_event::ChangeEvent;
pub use change_op::ChangeOp;
pub use fed_error::FedError;
pub use node_keypair::NodeKeypair;
pub use node_public_key::NodePublicKey;
pub use signed_part::SignedPart;
pub(crate) use from_hex::from_hex;
pub(crate) use to_hex::to_hex;

pub use binding::community_key_publish_challenge::community_key_publish_challenge;
pub use binding::community_keypair::CommunityKeypair;
pub use binding::community_public_key::CommunityPublicKey;
pub use binding::home_binding::HomeBinding;

pub use ownership::auth_error::AuthError;
pub use ownership::authorize::authorize;
pub use ownership::binding_is_authoritative::binding_is_authoritative;
pub use ownership::claim_outcome::ClaimOutcome;
pub use ownership::event_scope::event_scope;
pub use ownership::event_scope::EventScope;
pub use ownership::in_memory_registry::InMemoryRegistry;
pub use ownership::no_parents::NoParents;
pub use ownership::node_load::NodeLoad;
pub use ownership::node_status::NodeStatus;
pub use ownership::ownership::Ownership;
pub use ownership::ownership_registry::OwnershipRegistry;
pub use ownership::parent_demos::ParentDemos;
pub use ownership::parent_kind::ParentKind;
pub use ownership::registry_error::RegistryError;

pub use rehome::choose_new_owner::choose_new_owner;
pub use rehome::choose_new_standby::choose_new_standby;
pub use rehome::rehome_outcome::RehomeOutcome;
pub use rehome::rehoming_controller::RehomingController;
