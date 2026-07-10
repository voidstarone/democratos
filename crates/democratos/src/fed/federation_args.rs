//! Everything the federation runtime needs, gathered from CLI/env.

/// Everything the federation runtime needs, gathered from CLI/env.
pub struct FederationArgs {
    pub node_id: u16,
    /// Node-only address for the change-feed server (firewall to the node network).
    pub federation_addr: String,
    /// etcd endpoints; empty → in-process registry (single-node / dev).
    pub etcd_endpoints: Vec<String>,
    /// Shared node-to-node bearer token (`None` = open; only for trusted networks).
    pub cluster_token: Option<String>,
    /// Peers to replicate from, as `(node_id, base_url)`.
    pub peers: Vec<(i64, String)>,
    pub lease_ttl_secs: i64,
    pub poll_interval_secs: u64,
}
