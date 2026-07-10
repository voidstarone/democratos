use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::pager::Pager;
use crate::views::post_row::PostRow;
use crate::views::rule_view::RuleView;
use crate::views::standing_view::StandingView;

#[derive(Template)]
#[template(path = "demos.html")]
pub struct DemosView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub slug: String,
    pub name: String,
    pub phase: String,
    pub voters: u64,
    pub criteria_age: i64,
    pub criteria_member: i64,
    pub criteria_contrib: i64,
    /// True once the demos has left Seed and may amend its constitution.
    pub can_amend: bool,
    pub viewer_is_voter: bool,
    /// A member in good standing may post and comment.
    pub viewer_can_post: bool,
    pub standing: Option<StandingView>,
    pub rules: Vec<RuleView>,
    pub posts: Vec<PostRow>,
    pub pager: Pager,
}
