//! The template/JS token for a feed-delivery preference.

use domain::FeedPaging;

/// The template/JS token for a delivery preference.
pub(crate) fn paging_str(p: FeedPaging) -> &'static str {
    match p {
        FeedPaging::Auto => "auto",
        FeedPaging::Pages => "pages",
        FeedPaging::Lazy => "lazy",
    }
}
