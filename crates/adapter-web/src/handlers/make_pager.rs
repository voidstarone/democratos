//! Build the pager for a feed render.

use crate::views::pager::Pager;

/// Build the pager for a feed. `base` is the path plus a trailing query separator
/// (`?` if it carries no other params, `&` if it does), so appending `page=N`
/// yields a valid URL. `prev` is only populated for paged navigation.
pub(crate) fn make_pager(base: &str, page: usize, has_next: bool, mode: &'static str) -> Pager {
    Pager {
        mode,
        prev_href: (page > 1).then(|| format!("{base}page={}", page - 1)),
        next_href: has_next.then(|| format!("{base}page={}", page + 1)),
    }
}
