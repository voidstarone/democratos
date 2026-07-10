//! Normalize raw tag input into clean, deduped tags.

use crate::content::max_tags::MAX_TAGS;

/// Normalize raw tag input into clean, deduped tags. Splits on commas and
/// whitespace, lowercases, keeps only `[a-z0-9-]`, drops empties, and caps the
/// count at [`MAX_TAGS`]. Order of first appearance is preserved.
pub fn normalize_tags(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for piece in raw.split(|c: char| c == ',' || c.is_whitespace()) {
        let tag: String = piece
            .trim()
            .to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if tag.is_empty() || out.contains(&tag) {
            continue;
        }
        out.push(tag);
        if out.len() == MAX_TAGS {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_tags() {
        assert_eq!(normalize_tags("Rust, async  ,RUST"), vec!["rust", "async"]); // lowercase + dedupe
        assert_eq!(normalize_tags("c++ web-dev"), vec!["c", "web-dev"]); // strip punctuation, keep hyphen
        assert_eq!(normalize_tags("   "), Vec::<String>::new());
        assert_eq!(
            normalize_tags("a b c d e f g"),
            vec!["a", "b", "c", "d", "e"] // capped at MAX_TAGS (5)
        );
    }
}
