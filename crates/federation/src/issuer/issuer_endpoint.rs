//! A discovered trusted account issuer a node can forward account minting to.

use domain::NodeId;

use crate::NodeLoad;

/// A trusted account issuer discovered through the control plane: a live node that
/// holds a valid [`IssuerCert`](super::issuer_cert::IssuerCert) *and* has published
/// a reachable command address. `load` drives selection so minting spreads across
/// issuers rather than piling onto one.
#[derive(Clone, Debug, PartialEq)]
pub struct IssuerEndpoint {
    pub node: NodeId,
    /// The node's base URL for the federation command endpoint (from the registry).
    pub addr: String,
    pub load: NodeLoad,
}
