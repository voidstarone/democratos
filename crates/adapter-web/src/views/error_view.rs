use askama::Template;

use crate::i18n::strings::Strings;

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub message: String,
}
