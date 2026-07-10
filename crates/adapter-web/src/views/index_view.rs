use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::demos_list_item::DemosListItem;
use crate::views::founding_list_item::FoundingListItem;
use crate::views::pager::Pager;
use crate::views::post_row::PostRow;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    /// The home's syndicated feed: a signed-in viewer's recommendations, or the
    /// global-popular leaderboard when signed out (or as a fallback while the
    /// recommender has nothing to show yet).
    pub feed: Vec<PostRow>,
    pub pager: Pager,
    pub demos: Vec<DemosListItem>,
    /// Communities being founded — still gathering their nine sign-offs.
    pub foundings: Vec<FoundingListItem>,
}
