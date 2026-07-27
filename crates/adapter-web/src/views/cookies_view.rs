use askama::Template;

use crate::i18n::strings::Strings;

/// The cookie / privacy notice. A static disclosure page: it names every cookie
/// the app can set, what each is for and how long it lives, so the transparency
/// duty is met without a consent gate. There is deliberately no banner — every
/// cookie listed is either strictly necessary or set only on an explicit user
/// action, so none of them is consent-requiring, and a banner would imply
/// tracking that does not exist.
#[derive(Template)]
#[template(path = "cookies.html")]
pub struct CookiesView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
}
