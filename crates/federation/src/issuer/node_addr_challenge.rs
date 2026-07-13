//! The canonical bytes a node signs to authenticate its published address.

/// Canonical bytes a node signs (with its [`NodeKeypair`](crate::NodeKeypair)) to
/// prove it authorised its own advertised address. The registry verifies this
/// signature against the node's published key before ever handing the address out,
/// so a party with mere control-plane write access cannot poison a node's address to
/// redirect forwarded credentials (delegated login/minting) to a server it controls.
pub fn node_addr_challenge(node: u16, url: &str) -> String {
    format!("democratos:node-addr:v1;node:{node};url:{url}")
}
