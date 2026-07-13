//! Finish-signup form fields for an accepted invite.

use serde::Deserialize;

/// What the invite-accept finish form submits: the carried token, the chosen
/// handle + password, and the CSRF token. The email is not submitted — it is
/// fixed by the token server-side, so a visitor can't retarget the invite.
#[derive(Deserialize)]
pub struct AcceptInviteForm {
    pub(crate) token: String,
    pub(crate) handle: String,
    pub(crate) password: String,
    #[serde(default)]
    pub(crate) csrf_token: String,
}
