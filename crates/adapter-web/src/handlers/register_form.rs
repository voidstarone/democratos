//! Real sign-up form fields.

use serde::Deserialize;

/// Real sign-up: a handle to be known by, plus login credentials and CSRF token.
#[derive(Deserialize)]
pub struct RegisterForm {
    pub(crate) handle: String,
    pub(crate) email: String,
    pub(crate) password: String,
    #[serde(default)]
    pub(crate) csrf_token: String,
}
