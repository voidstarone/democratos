//! Derive a URL slug from a community's display name.

/// The maximum length of a derived slug, in characters. Long enough to stay
/// readable, short enough to keep URLs and mentions tidy.
const MAX_SLUG_LEN: usize = 48;

/// Derive a URL slug from a community's display name: lowercase, ASCII
/// alphanumerics kept, every other run collapsed to a single hyphen, and the
/// ends trimmed of hyphens. Returns an empty string when the name has no usable
/// characters (e.g. all punctuation or non-ASCII) — callers treat that as an
/// invalid name rather than founding a slugless community.
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len().min(MAX_SLUG_LEN + 1));
    let mut pending_hyphen = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            pending_hyphen = false;
            slug.extend(ch.to_lowercase());
        } else {
            // Any non-alphanumeric (space, punctuation, non-ASCII) becomes a
            // separator; runs collapse because we only emit on the next kept char.
            pending_hyphen = true;
        }
    }
    // The slug is pure ASCII, so a byte truncation is a char boundary. Trim any
    // hyphen the cut left dangling so it never ends on a separator.
    if slug.len() > MAX_SLUG_LEN {
        slug.truncate(MAX_SLUG_LEN);
        while slug.ends_with('-') {
            slug.pop();
        }
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basics() {
        assert_eq!(slugify("Rustaceans"), "rustaceans");
        assert_eq!(slugify("Rust Fans"), "rust-fans");
        assert_eq!(slugify("  Hello,  World!  "), "hello-world");
        assert_eq!(slugify("C++ / Systems"), "c-systems");
        assert_eq!(slugify("already-a-slug"), "already-a-slug");
    }

    #[test]
    fn slugify_edges() {
        // No usable characters yields an empty slug (an invalid name).
        assert_eq!(slugify("!!!"), "");
        assert_eq!(slugify(""), "");
        // Leading/trailing separators never leak into the slug.
        assert_eq!(slugify("-mid--dle-"), "mid-dle");
        // Overlong names are truncated without a trailing hyphen.
        let long = slugify(&"ab ".repeat(40));
        assert!(long.len() <= MAX_SLUG_LEN);
        assert!(!long.ends_with('-'));
    }
}
