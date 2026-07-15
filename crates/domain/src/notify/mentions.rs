//! Extract `@handle` mentions from a body of text.

/// The distinct handles mentioned as `@handle` in `text`, lowercased and
/// de-duplicated, in first-seen order. A mention runs from `@` over the handle
/// characters `[A-Za-z0-9_-]` (matching how handles are written); an `@` not
/// preceded by whitespace/start — e.g. inside an email address — is ignored so
/// `a@b.com` doesn't read as a mention of `b`. The caller resolves each handle to
/// an account (and drops self-mentions / unknown handles).
pub fn mentions(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            // Only a boundary `@` starts a mention: preceded by start-of-text or a
            // non-handle character (whitespace, punctuation). This rules out the
            // `@` in the middle of a token such as an email address.
            let at_boundary = i == 0 || !is_handle_byte(bytes[i - 1]);
            if at_boundary {
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && is_handle_byte(bytes[end]) {
                    end += 1;
                }
                if end > start {
                    let handle = text[start..end].to_lowercase();
                    if !found.contains(&handle) {
                        found.push(handle);
                    }
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    found
}

fn is_handle_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_distinct_handles_in_order() {
        assert_eq!(
            mentions("hi @alice and @bob, thanks @alice"),
            vec!["alice", "bob"]
        );
    }

    #[test]
    fn is_case_insensitive() {
        assert_eq!(mentions("@Alice @ALICE @alice"), vec!["alice"]);
    }

    #[test]
    fn ignores_an_at_inside_a_token_like_an_email() {
        assert_eq!(mentions("mail me at bob@example.com"), Vec::<String>::new());
    }

    #[test]
    fn a_bare_at_is_not_a_mention() {
        assert_eq!(mentions("cost @ $5 each @"), Vec::<String>::new());
    }

    #[test]
    fn stops_at_punctuation() {
        assert_eq!(mentions("(@alice) said @bob!"), vec!["alice", "bob"]);
    }
}
