//! Public invite-request form fields.

use serde::Deserialize;

/// What the public waitlist form submits: an email, an optional note, and the
/// CSRF token.
#[derive(Deserialize)]
pub struct RequestInviteForm {
    pub(crate) email: String,
    #[serde(default)]
    pub(crate) note: String,
    #[serde(default)]
    pub(crate) csrf_token: String,
}
