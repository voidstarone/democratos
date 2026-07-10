//! Founder-signed home-node bindings — the per-community cryptographic anchor that
//! makes "the founder chose THIS host" enforceable in an **open federation of
//! untrusted node operators**.
//!
//! Every community has a [`CommunityKeypair`](community_keypair::CommunityKeypair)
//! minted at founding. Its home node holds the private half and publishes the public
//! half to the control plane. The home node signs a
//! [`HomeBinding`](home_binding::HomeBinding) naming the community's `home_node` and
//! the set of nodes pre-authorized to take over during the home's downtime
//! (`allowed_failover`, which lets the *hybrid failover* happen without the
//! founder online). Peers verify — in [`crate::authorize`] and before honouring a
//! `claim` — that whoever owns a community is authorized by its binding.
//!
//! This is what closes the takeover hole: a node that grabs the etcd holder key it
//! does not deserve is still rejected fleet-wide, because it is not named in the
//! founder-signed binding and cannot forge one without the community's secret key.
//! The ID-origin is only a bootstrap hint; the binding is the authority, and it can
//! be re-signed (higher `epoch`) to migrate a community to a new home.

mod binding_body;
pub mod community_key_publish_challenge;
pub mod community_keypair;
pub mod community_public_key;
pub mod home_binding;
