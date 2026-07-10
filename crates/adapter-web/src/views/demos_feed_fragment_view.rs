use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::pager::Pager;
use crate::views::post_row::PostRow;

/// The lazy-load slice for a community feed. Carries the community slug and the
/// viewer's voter status so appended cards keep the "propose removal" action.
#[derive(Template)]
#[template(path = "demos_fragment.html")]
pub struct DemosFeedFragmentView {
    pub t: Strings,
    pub posts: Vec<PostRow>,
    pub pager: Pager,
    pub slug: String,
    pub viewer_is_voter: bool,
}
