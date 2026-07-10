use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::auth_mode::AuthMode;

#[derive(Template)]
#[template(path = "signin.html")]
pub struct SignInView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub mode: AuthMode,
    /// Anti-CSRF token, echoed into the form as a hidden field and matched
    /// against the `csrf` cookie on submit (double-submit defence against login
    /// CSRF).
    pub csrf_token: String,
}
