//! Periodic re-claim + load-refresh heartbeat.

use std::sync::Arc;
use std::time::Duration;

use adapter_store_postgres::PostgresStore;
use domain::NodeId;
use federation::{NodeKeypair, OwnershipRegistry};

use crate::fed::claim_hosted::claim_hosted;

/// Periodically re-claim (picks up communities founded after boot, and re-asserts
/// ownership after a transient registry blip) and refresh reported load.
pub(crate) fn spawn_maintenance(
    store: Arc<PostgresStore>,
    registry: Arc<dyn OwnershipRegistry>,
    node: NodeId,
    keypair: Arc<NodeKeypair>,
    lease_ttl_secs: i64,
) {
    let period = Duration::from_secs((lease_ttl_secs / 2).max(1) as u64);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(period);
        loop {
            tick.tick().await;
            if let Err(e) = claim_hosted(&store, registry.as_ref(), node, keypair.as_ref()).await {
                eprintln!("⚠ federation: maintenance tick failed: {e}");
            }
        }
    });
}
