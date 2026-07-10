use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::demos_list_item::DemosListItem;
use crate::views::pager::Pager;
use crate::views::post_row::PostRow;

#[derive(Template)]
#[template(path = "search.html")]
pub struct SearchView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub query: String,
    /// `Some(slug)` when the search is scoped to one community; drives the
    /// scope toggle and the "search here vs everywhere" links.
    pub scope_slug: Option<String>,
    pub tag: Option<String>,
    pub communities: Vec<DemosListItem>,
    pub posts: Vec<PostRow>,
    pub pager: Pager,
}
