//! The control-plane contract: who owns which community, at which epoch — and the
//! full **authorization** of a change event that this makes possible.
//!
//! # Why this is the security keystone
//!
//! A signature proves an event is *authentic* (it really came from the node whose
//! key signed it). It does **not** prove the event is *authorized* — that the
//! signing node is the community's rightful owner, and that it spoke under the
//! current ownership epoch rather than a stale one it lost in a failover. Without
//! that second check, any node whose key a peer trusts could forge authoritative
//! state for **any** community, splitting the electorate and defeating the whole
//! anti-takeover design (review finding #1).
//!
//! [`authorize`](crate::authorize) closes that gap by combining the signature
//! check with an ownership+epoch check against an
//! [`OwnershipRegistry`](crate::OwnershipRegistry) (etcd in production, the
//! [`InMemoryRegistry`](crate::InMemoryRegistry) in tests/dev). Ownership is a
//! **lease**: the owner renews it on a heartbeat; if it lapses (the node went
//! down), a peer may [`claim`](crate::OwnershipRegistry::claim) the community,
//! which **bumps the epoch**. The bump is the fencing token — a returning old owner
//! still holds the previous epoch, so its events are rejected as
//! [`AuthError::StaleEpoch`](crate::AuthError::StaleEpoch). Epochs are monotonic
//! and survive owner death.

pub mod auth_error;
pub mod authorize;
pub mod binding_is_authoritative;
pub mod claim_outcome;
pub mod event_scope;
pub mod in_memory_registry;
pub mod no_parents;
pub mod node_load;
pub mod node_status;
#[allow(clippy::module_inception)]
pub mod ownership;
pub mod ownership_registry;
pub mod parent_demos;
pub mod parent_kind;
pub mod registry_error;
