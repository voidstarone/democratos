use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::pager::Pager;
use crate::views::post_row::PostRow;

/// The bare feed slice returned to the JS lazy-loader for a post feed (home,
/// `/top`, a community). It renders only the post cards plus the next-page
/// control — no page chrome — so the client can append them in place.
#[derive(Template)]
#[template(path = "feed_fragment.html")]
pub struct FeedFragmentView {
    pub t: Strings,
    pub posts: Vec<PostRow>,
    pub pager: Pager,
}
