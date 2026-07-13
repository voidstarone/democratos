//! Validate a post-auth `next` redirect target.

/// Sanitise a post-auth `next` target to a safe **same-site** path, or `None`.
///
/// It must be a root-relative path (`/…`) drawn from a conservative allowlist —
/// alphanumerics and `/-._~` only. That rules out an absolute or protocol-relative
/// URL (`//evil`, `https://evil`), a backslash trick (`/\evil`), CR/LF header
/// injection, and any query/fragment that could break the value out of an `href`.
/// Anything else returns `None` so the caller falls back to `/`, closing the
/// open-redirect hole where a crafted `?next=` bounces a fresh sign-in off-origin.
pub(crate) fn safe_next(next: &str) -> Option<String> {
    let n = next.trim();
    if !n.starts_with('/') || n.starts_with("//") {
        return None;
    }
    if !n
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~'))
    {
        return None;
    }
    Some(n.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_local_paths_only() {
        assert_eq!(safe_next("/found/12"), Some("/found/12".to_string()));
        assert_eq!(safe_next("/d/rust-lang"), Some("/d/rust-lang".to_string()));
        // open-redirect / injection attempts are refused
        assert_eq!(safe_next("//evil.com"), None);
        assert_eq!(safe_next("https://evil.com"), None);
        assert_eq!(safe_next("/\\evil.com"), None);
        assert_eq!(safe_next("/a?b=c"), None);
        assert_eq!(safe_next("/a\r\nSet-Cookie: x"), None);
        assert_eq!(safe_next("relative"), None);
        assert_eq!(safe_next(""), None);
    }
}
