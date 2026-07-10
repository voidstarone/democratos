//! How many items one feed page carries.

/// How many items one feed page carries. Feeds render this many, plus a control
/// to reach the next page (a plain link with no JS, lazy-loaded with it).
pub(crate) const FEED_PAGE_SIZE: usize = 20;
