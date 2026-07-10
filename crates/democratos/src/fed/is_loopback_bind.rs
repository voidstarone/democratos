//! Whether a bind address is loopback-only.

use std::net::{IpAddr, SocketAddr};

/// Whether a bind address is loopback-only (so the federation port is
/// unreachable from off-box). Used to decide when a cluster token is mandatory.
///
/// A parseable loopback IP (`127.0.0.0/8`, `::1`) or the `localhost` hostname is
/// loopback. `0.0.0.0` / `::` (all interfaces) and any routable IP are not.
/// Anything we can't confidently classify (an unresolved hostname) is treated as
/// **not** loopback — fail closed, so an ambiguous address still requires a token.
pub(crate) fn is_loopback_bind(addr: &str) -> bool {
    if let Ok(sock) = addr.parse::<SocketAddr>() {
        return sock.ip().is_loopback();
    }
    // Not a full `ip:port` — strip the port and inspect the host.
    let host = match addr.rsplit_once(':') {
        Some((h, _port)) => h.trim_matches(['[', ']']),
        None => addr,
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_loopback_binds() {
        for a in [
            "127.0.0.1:7000",
            "127.0.0.5:7000",
            "[::1]:7000",
            "localhost:7000",
            "LOCALHOST:7000",
        ] {
            assert!(is_loopback_bind(a), "{a} should be loopback");
        }
        for a in [
            "0.0.0.0:7000",
            "[::]:7000",
            "10.0.0.5:7000",
            "192.168.1.9:7000",
            "node.internal:7000",
        ] {
            assert!(!is_loopback_bind(a), "{a} should NOT be loopback");
        }
    }
}
