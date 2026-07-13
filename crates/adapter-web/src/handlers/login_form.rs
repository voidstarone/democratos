//! Real sign-in form fields.

use serde::Deserialize;

/// Real sign-in: handle + password, plus the anti-CSRF token from the form. Login is
/// by handle (not email) because handles replicate across the federation while emails
/// are redacted — so a community node can resolve the account and route the login to
/// its home issuer, whereas it never sees the email.
#[derive(Deserialize)]
pub struct LoginForm {
    pub(crate) handle: String,
    pub(crate) password: String,
    /// The double-submit CSRF token; validated against the `csrf` cookie. Defaults
    /// to empty (which fails validation) so a token-less POST is rejected, not a 400.
    #[serde(default)]
    pub(crate) csrf_token: String,
    /// Optional same-site return path (validated via `safe_next` on submit).
    #[serde(default)]
    pub(crate) next: String,
}
