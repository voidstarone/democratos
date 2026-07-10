use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::pager::Pager;
use crate::views::post_row::PostRow;

#[derive(Template)]
#[template(path = "top.html")]
pub struct TopView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub posts: Vec<PostRow>,
    pub pager: Pager,
}
