use askama::Template;

use crate::i18n::strings::Strings;

/// The public "ask for an invite" page. Anyone with the link can reach it; it
/// takes an email (and an optional note) onto the waitlist.
#[derive(Template)]
#[template(path = "request_invite.html")]
pub struct RequestInviteView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub csrf_token: String,
    /// Set after a successful submit so the page confirms and hides the form.
    pub submitted: bool,
    /// A validation message to show inline (e.g. a malformed email); `None` on
    /// first render.
    pub error: Option<String>,
    /// What the visitor typed, echoed back so a rejected submit isn't cleared.
    pub email: String,
    pub note: String,
}
