use crate::http::peer::Peer;
use crate::Replicator;

/// Pull one page from `peer` and apply the authorized events. Returns how many
/// were applied. Resumes from this node's stored cursor for the peer.
pub async fn poll_peer(
    replicator: &Replicator,
    peer: &Peer,
    limit: i64,
) -> app::Result<u64> {
    let since = replicator.cursor(peer.node).await?;
    let events = peer
        .client
        .changes_since(since, limit)
        .await
        .map_err(|e| app::StoreError::Store(e.to_string()))?;
    if events.is_empty() {
        return Ok(0);
    }
    let out = replicator.ingest(peer.node, &events).await?;
    for why in &out.rejected {
        tracing::warn!(peer = peer.node, "rejected federated event: {why}");
    }
    Ok(out.applied)
}
