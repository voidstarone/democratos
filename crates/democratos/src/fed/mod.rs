//! Composition-root assembly of the federation runtime.
//!
//! This is the only place that names the concrete control-plane adapter (etcd vs.
//! the in-process registry) and stitches the tested federation parts into a live
//! node: it loads this node's signing identity, publishes its public key, claims
//! the communities it hosts, starts the change-feed server (on a node-only
//! address) and the peer puller, and keeps ownership + load fresh on a heartbeat.
//!
//! It deliberately does *not* contain any federation logic — signing,
//! authorization, apply, and transport all live in `federation` /
//! `adapter-federation`. This module only wires them to configuration.

pub mod claim_hosted;
pub mod federation_args;
pub mod guard_federation_exposure;
pub mod is_loopback_bind;
pub mod parse_endpoints;
pub mod parse_peer;
pub mod require_tls_for_remote;
pub mod spawn_maintenance;
pub mod spawn_rehoming;
pub mod start;
