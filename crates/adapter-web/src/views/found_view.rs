use askama::Template;

use crate::i18n::strings::Strings;

/// The dedicated "found a community" page. The founder supplies only a display
/// name; the slug is derived from it. Submitting opens a founding petition
/// rather than creating a demos outright.
#[derive(Template)]
#[template(path = "found.html")]
pub struct FoundView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
}
