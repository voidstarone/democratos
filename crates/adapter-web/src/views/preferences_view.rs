use askama::Template;

use crate::i18n::strings::Strings;

/// The account preferences page. Today it carries just the feed-delivery choice.
#[derive(Template)]
#[template(path = "preferences.html")]
pub struct PreferencesView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    /// The saved delivery preference: `"auto"`, `"pages"`, or `"lazy"` — drives
    /// which radio is checked.
    pub feed_paging: &'static str,
    /// Set after a successful save so the page can confirm it.
    pub saved: bool,
}
