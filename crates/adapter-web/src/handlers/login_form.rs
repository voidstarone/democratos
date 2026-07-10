//! Real sign-in form fields.

use serde::Deserialize;

/// Real sign-in: email + password, plus the anti-CSRF token from the form.
#[derive(Deserialize)]
pub struct LoginForm {
    pub(crate) email: String,
    pub(crate) password: String,
    /// The double-submit CSRF token; validated against the `csrf` cookie. Defaults
    /// to empty (which fails validation) so a token-less POST is rejected, not a 400.
    #[serde(default)]
    pub(crate) csrf_token: String,
}
