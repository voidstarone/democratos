//! Periodic failover: take over communities this node is the best standby for.

use std::sync::Arc;
use std::time::Duration;

use adapter_store_postgres::PostgresStore;
use app::DemosStore;
use domain::NodeId;
use federation::{OwnershipRegistry, RehomeOutcome, RehomingController};

/// Periodically take over any community this node replicates that has lost its
/// owner (lease lapsed) and for which this node is the best (quietest) standby.
/// The claim bumps the epoch, fencing the old owner if it returns.
pub(crate) fn spawn_rehoming(
    store: Arc<PostgresStore>,
    registry: Arc<dyn OwnershipRegistry>,
    node: NodeId,
    lease_ttl_secs: i64,
) {
    let controller = RehomingController::new(node, registry);
    // Wait roughly a full lease before reacting, so a brief blip isn't a failover.
    let period = Duration::from_secs(lease_ttl_secs.max(1) as u64);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(period);
        loop {
            tick.tick().await;
            let ids: Vec<u64> = match DemosStore::list(&*store).await {
                Ok(demoi) => demoi.iter().map(|d| d.id.0).collect(),
                Err(e) => {
                    eprintln!("⚠ federation: rehoming could not list communities: {e}");
                    continue;
                }
            };
            for outcome in controller.tick(&ids).await {
                match outcome {
                    RehomeOutcome::Promoted { demos, epoch } => eprintln!(
                        "federation: rehomed community {demos} onto this node (epoch {epoch})"
                    ),
                    RehomeOutcome::Stranded { demos } => eprintln!(
                        "⚠ federation: community {demos} is unowned and has no live standby"
                    ),
                    _ => {}
                }
            }
        }
    });
}
