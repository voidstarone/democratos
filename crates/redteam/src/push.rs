//! Push forged events to an honest node's ingest endpoint.

use anyhow::{anyhow, Result};

use adapter_federation::IngestClient;
use federation::ChangeEvent;

pub(crate) async fn push(
    feed: &str,
    token: Option<String>,
    peer_node: i64,
    events: &[ChangeEvent],
) -> Result<u64> {
    let client = IngestClient::new(feed.to_string(), token);
    client
        .push(peer_node, events)
        .await
        .map_err(|e| anyhow!("ingest push to {feed} failed: {e:?}"))
}
