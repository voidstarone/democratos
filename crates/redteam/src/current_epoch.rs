//! The current ownership epoch of a community.

use adapter_control_etcd::EtcdRegistry;
use federation::OwnershipRegistry;

/// The current ownership epoch of a community (1 if unowned), so forged events are
/// signed at a non-stale epoch — isolating the ownership check as the rejecter.
pub(crate) async fn current_epoch(reg: &EtcdRegistry, demos: u64) -> u64 {
    reg.owner_of(demos)
        .await
        .ok()
        .flatten()
        .map(|o| o.epoch)
        .unwrap_or(1)
}
