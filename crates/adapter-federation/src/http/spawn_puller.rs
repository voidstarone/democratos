use std::sync::Arc;
use std::time::Duration;

use crate::http::peer::Peer;
use crate::http::poll_peer::poll_peer;
use crate::Replicator;

/// Spawn a background loop that polls every peer on `interval`, forever. A failed
/// poll is logged and retried next tick — a down or misbehaving peer never takes
/// this node down.
pub fn spawn_puller(
    replicator: Arc<Replicator>,
    peers: Vec<Peer>,
    interval: Duration,
    limit: i64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval.max(Duration::from_secs(1)));
        loop {
            tick.tick().await;
            for peer in &peers {
                match poll_peer(&replicator, peer, limit).await {
                    Ok(n) if n > 0 => tracing::info!(peer = peer.node, applied = n, "replicated"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!(peer = peer.node, "poll failed: {e}"),
                }
            }
        }
    })
}
