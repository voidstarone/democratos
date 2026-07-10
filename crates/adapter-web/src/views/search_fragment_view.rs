use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::pager::Pager;
use crate::views::post_row::PostRow;

/// The lazy-load slice for the search results feed (its cards differ from the
/// post-feed cards, so it has its own fragment).
#[derive(Template)]
#[template(path = "search_fragment.html")]
pub struct SearchFragmentView {
    pub t: Strings,
    pub posts: Vec<PostRow>,
    pub pager: Pager,
}
