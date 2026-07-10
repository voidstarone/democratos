//! Take one page-sized window out of an already-ordered list.

use crate::handlers::feed_page_size::FEED_PAGE_SIZE;

/// Take the `page`-th window of `FEED_PAGE_SIZE` items from an already-ordered
/// list, returning the slice and whether any items remain beyond it (so the
/// caller knows whether to offer a "next page"). A page past the end is empty.
pub(crate) fn paginate<T>(items: Vec<T>, page: usize) -> (Vec<T>, bool) {
    let start = (page - 1) * FEED_PAGE_SIZE;
    let has_next = items.len() > start + FEED_PAGE_SIZE;
    let window = items
        .into_iter()
        .skip(start)
        .take(FEED_PAGE_SIZE)
        .collect();
    (window, has_next)
}
