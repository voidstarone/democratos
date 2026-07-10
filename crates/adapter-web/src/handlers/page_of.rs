//! Normalise a `?page=` query value to a 1-based page number.

/// Normalise a `?page=` query value to a 1-based page number (absent or `0` → 1).
pub(crate) fn page_of(page: Option<u32>) -> usize {
    page.unwrap_or(1).max(1) as usize
}
