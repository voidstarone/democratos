use crate::http::feed_client::FeedClient;

/// One configured peer to replicate from.
pub struct Peer {
    /// The peer's node id — the replication cursor key.
    pub node: i64,
    pub client: FeedClient,
}
