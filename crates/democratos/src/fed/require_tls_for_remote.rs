//! Reject a plaintext, off-box federation / control-plane URL.

use anyhow::{bail, Result};

use crate::fed::is_loopback_bind::is_loopback_bind;

/// Reject a plaintext, off-box federation / control-plane URL unless the operator
/// explicitly opts into an isolated trusted network. Peer, command, ingest and
/// etcd links carry the shared cluster token and the entire replicated dataset;
/// over a non-TLS link a network attacker captures the token and can read every
/// community's change feed. (Event *forgery* is still blocked by the Ed25519
/// envelope; this is about confidentiality and token capture.) Loopback is always
/// exempt — nothing leaves the box.
pub(crate) fn require_tls_for_remote(kind: &str, url: &str) -> Result<()> {
    let url = url.trim();
    let host = url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url);
    if is_loopback_bind(host) || url.starts_with("https://") {
        return Ok(());
    }
    if std::env::var_os("DEMOCRATOS_ALLOW_PLAINTEXT_FEDERATION").is_some() {
        eprintln!(
            "⚠  federation: {kind} {url:?} is plaintext (no TLS). The cluster token and all \
             replicated data cross this link in the clear. Permitted only because \
             DEMOCRATOS_ALLOW_PLAINTEXT_FEDERATION is set — use it solely on an isolated, \
             trusted network."
        );
        return Ok(());
    }
    bail!(
        "refusing a plaintext {kind} URL {url:?}: it carries the cluster token and replicated \
         community data, which a network attacker on a non-TLS link can capture. Use https:// \
         (terminate TLS at the peer, e.g. behind the bundled Caddy). For an isolated, trusted \
         network only, override with DEMOCRATOS_ALLOW_PLAINTEXT_FEDERATION=1."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_required_for_remote_links() {
        // Loopback is exempt regardless of scheme (nothing leaves the box).
        assert!(require_tls_for_remote("peer", "http://127.0.0.1:7400").is_ok());
        assert!(require_tls_for_remote("peer", "http://localhost:7400").is_ok());
        // https to a remote is fine.
        assert!(require_tls_for_remote("peer", "https://node.internal:7400").is_ok());
        // Plaintext to a remote is refused (the escape-hatch env is unset here).
        assert!(require_tls_for_remote("peer", "http://10.0.0.5:7400").is_err());
        assert!(require_tls_for_remote("etcd endpoint", "http://node.internal:2379").is_err());
        assert!(require_tls_for_remote("peer", "10.0.0.5:7400").is_err());
    }
}
