//! The insecure-connection check for a Postgres URL.

/// Whether `url` connects to Postgres without TLS. Federation nodes talk to a
/// remote database over an untrusted network, so an un-encrypted link would
/// expose (and allow tampering with) every replicated row and vote in flight.
/// A `localhost`/`127.0.0.1`/unix-socket target is treated as safe; anything
/// else without `sslmode=require` (or stricter) is flagged.
pub fn is_insecure_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    // A unix-socket connection ("host=/var/run/...") never leaves the box.
    if lower.contains("host=/") || lower.starts_with("postgres:///") {
        return false;
    }
    // sslmode=require|verify-ca|verify-full all negotiate TLS.
    let tls_requested = lower.contains("sslmode=require")
        || lower.contains("sslmode=verify-ca")
        || lower.contains("sslmode=verify-full");
    if tls_requested {
        return false;
    }
    // Loopback / same-host targets don't cross the network.
    let host = host_of(&lower);
    let loopback = matches!(
        host.as_deref(),
        Some("localhost" | "127.0.0.1" | "::1" | "")
    ) || host.is_none();
    !loopback
}

/// Best-effort extraction of the host from a `postgres://[user[:pw]@]host[:port]/db`
/// URL (only used by [`is_insecure_url`]; not a general URL parser).
fn host_of(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1)?;
    let authority = after_scheme.split(['/', '?']).next()?;
    let hostport = authority.rsplit('@').next()?;
    // Bracketed IPv6 `[::1]:5432`.
    if let Some(rest) = hostport.strip_prefix('[') {
        return rest.split(']').next().map(str::to_string);
    }
    hostport.split(':').next().map(str::to_string)
}

#[cfg(test)]
mod url_tests {
    use super::{host_of, is_insecure_url};

    #[test]
    fn loopback_and_socket_targets_are_safe() {
        for url in [
            "postgres://u:p@localhost/db",
            "postgres://u:p@127.0.0.1:5432/db",
            "postgres://u@[::1]:5432/db",
            "postgres:///db?host=/var/run/postgresql",
            "postgres://u:p@localhost:5432/db?application_name=x",
        ] {
            assert!(!is_insecure_url(url), "{url} should be treated as safe");
        }
    }

    #[test]
    fn remote_without_tls_is_insecure() {
        for url in [
            "postgres://u:p@db.internal:5432/democratos",
            "postgres://u:p@10.0.0.5/democratos",
            "postgres://u:p@db.internal/democratos?sslmode=disable",
        ] {
            assert!(is_insecure_url(url), "{url} should be flagged insecure");
        }
    }

    #[test]
    fn remote_with_tls_is_safe() {
        for url in [
            "postgres://u:p@db.internal:5432/democratos?sslmode=require",
            "postgres://u:p@db.internal/democratos?sslmode=verify-full",
            "postgres://u:p@10.0.0.5/democratos?sslmode=verify-ca",
        ] {
            assert!(
                !is_insecure_url(url),
                "{url} requests TLS and should be safe"
            );
        }
    }

    #[test]
    fn host_is_parsed_from_authority() {
        assert_eq!(
            host_of("postgres://u:p@db.internal:5432/x").as_deref(),
            Some("db.internal")
        );
        assert_eq!(
            host_of("postgres://db.internal/x").as_deref(),
            Some("db.internal")
        );
        assert_eq!(
            host_of("postgres://u@[2001:db8::1]:5432/x").as_deref(),
            Some("2001:db8::1")
        );
    }
}
