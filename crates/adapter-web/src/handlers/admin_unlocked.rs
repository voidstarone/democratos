//! The gate for the admin review queue: subnet allowlist **and** a shared secret.

use std::net::IpAddr;

use app::constant_time_eq;

use crate::AppState;

/// Whether this caller may reach the admin review queue. Both gates must pass:
///
/// 1. A secret is configured *and* the supplied `key` matches it (constant-time).
///    With no secret set the queue is disabled outright — never reachable by
///    network position alone.
/// 2. The connection peer is loopback, or inside one of the configured admin
///    subnets.
///
/// A closed gate becomes a `404` at the call site, so the queue is neither
/// reachable nor discoverable from anywhere else.
pub(crate) fn admin_unlocked(state: &AppState, peer: IpAddr, key: &str) -> bool {
    let Some(secret) = state.admin_secret.as_deref() else {
        return false;
    };
    if !constant_time_eq(key.as_bytes(), secret.as_bytes()) {
        return false;
    }
    peer.is_loopback() || state.admin_subnets.iter().any(|net| net.contains(&peer))
}
