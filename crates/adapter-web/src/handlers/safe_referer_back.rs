//! A safe same-origin "go back" target derived from the `Referer` header.

use axum::http::{header, HeaderMap};

/// A safe "go back" target derived from the `Referer` header. Any absolute URL is
/// reduced to its path (+query) so the redirect always lands on *our* origin, and
/// anything we can't reduce to a rooted local path falls back to `/`. This stops a
/// crafted `Referer` turning a post-action redirect into an open redirect off-site.
pub(crate) fn safe_referer_back(headers: &HeaderMap) -> String {
    let raw = headers
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("/");
    local_path_of(raw).unwrap_or_else(|| "/".to_string())
}

/// Reduce a `Referer` value to a same-origin relative path. An absolute URL keeps
/// only the path after its authority; a bare rooted path is kept as-is. Returns
/// `None` for anything else (protocol-relative `//host`, a scheme with no path, a
/// non-rooted or non-URL value), so the caller can fall back to `/`.
fn local_path_of(referer: &str) -> Option<String> {
    let s = referer.trim();
    if let Some((_scheme, rest)) = s.split_once("://") {
        // Absolute URL: keep from the first '/' of the path onward, dropping the
        // scheme + authority. Reject a protocol-relative-looking path.
        let path = rest.get(rest.find('/')?..)?;
        return (!path.starts_with("//")).then(|| path.to_string());
    }
    (s.starts_with('/') && !s.starts_with("//") && !s.starts_with("/\\")).then(|| s.to_string())
}

#[cfg(test)]
mod redirect_tests {
    use super::local_path_of;

    #[test]
    fn absolute_same_or_other_origin_is_reduced_to_its_path() {
        assert_eq!(
            local_path_of("https://demos.example/d/rust"),
            Some("/d/rust".into())
        );
        // Even an attacker's host is reduced to just the path, so we stay on-origin.
        assert_eq!(
            local_path_of("https://evil.example/d/rust?x=1"),
            Some("/d/rust?x=1".into())
        );
        assert_eq!(local_path_of("http://evil.example/"), Some("/".into()));
    }

    #[test]
    fn rooted_relative_paths_pass_through() {
        assert_eq!(local_path_of("/post/42"), Some("/post/42".into()));
        assert_eq!(
            local_path_of("/d/rust?tag=x#frag"),
            Some("/d/rust?tag=x#frag".into())
        );
    }

    #[test]
    fn off_site_and_malformed_targets_are_refused() {
        // Protocol-relative — the classic open-redirect vector.
        assert_eq!(local_path_of("//evil.example/steal"), None);
        assert_eq!(local_path_of("https://evil.example//steal"), None);
        assert_eq!(local_path_of("/\\evil.example"), None);
        // A bare scheme with no path, a non-rooted value, and junk.
        assert_eq!(local_path_of("https://evil.example"), None);
        assert_eq!(local_path_of("javascript:alert(1)"), None);
        assert_eq!(local_path_of("evil.example/x"), None);
    }
}
