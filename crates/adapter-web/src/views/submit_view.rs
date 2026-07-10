use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::community_option::CommunityOption;

#[derive(Template)]
#[template(path = "submit.html")]
pub struct SubmitView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    /// Communities the signed-in user may post in.
    pub communities: Vec<CommunityOption>,
}
