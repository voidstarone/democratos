//! Reject an exposed federation port with no cluster token.

use anyhow::{bail, Result};

use crate::fed::is_loopback_bind::is_loopback_bind;

/// Reject an exposed federation port with no cluster token. The command and
/// ingest endpoints execute writes and apply pushed events on behalf of any user;
/// with no token, anyone who can reach the port can forge them. This is safe only
/// on a firewalled node-only network — so refuse to bind anywhere but loopback
/// unless a token is set, turning the deployment requirement into an enforced one.
pub(crate) fn guard_federation_exposure(federation_addr: &str, token: Option<&str>) -> Result<()> {
    let has_token = token.map(|t| !t.is_empty()).unwrap_or(false);
    if !has_token && !is_loopback_bind(federation_addr) {
        bail!(
            "refusing to serve federation on {federation_addr} without a cluster token: the \
             command/ingest endpoints are unauthenticated and would let anyone on the network \
             forge writes. Set --cluster-token (DEMOCRATOS_CLUSTER_TOKEN), or bind a loopback \
             address for local/dev."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_requires_token_when_exposed() {
        // Exposed with no token: rejected. Unresolvable host: fail closed too.
        assert!(guard_federation_exposure("0.0.0.0:7000", None).is_err());
        assert!(guard_federation_exposure("0.0.0.0:7000", Some("")).is_err());
        assert!(guard_federation_exposure("node.internal:7000", None).is_err());
        // Exposed WITH a token: allowed.
        assert!(guard_federation_exposure("0.0.0.0:7000", Some("s3cret")).is_ok());
        // Loopback: allowed with or without a token.
        assert!(guard_federation_exposure("127.0.0.1:7000", None).is_ok());
        assert!(guard_federation_exposure("[::1]:7000", None).is_ok());
    }
}
