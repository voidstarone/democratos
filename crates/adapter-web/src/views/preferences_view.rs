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
    /// Whether the account has opted in to sensitive-content review — drives the
    /// checkbox state.
    pub is_sensitive_reviewer: bool,
    /// Whether the account wants mention notifications — checkbox state.
    pub allows_mention_alerts: bool,
    /// Whether the account wants jury-summons notifications — checkbox state.
    pub allows_jury_alerts: bool,
    /// Whether the account wants trial-comment notifications — checkbox state.
    pub allows_trial_comment_alerts: bool,
    /// Set after a successful save so the page can confirm it.
    pub saved: bool,
}
