use askama::Template;

use crate::i18n::strings::Strings;

/// The "finish setting up your account" page reached from a valid invite link.
/// The email is fixed by the invite (shown read-only); the visitor picks a handle
/// and password.
#[derive(Template)]
#[template(path = "accept_invite.html")]
pub struct AcceptInviteView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    /// The invited email, bound by the token — displayed but not editable.
    pub email: String,
    /// The raw token, carried through the finish form as a hidden field.
    pub token: String,
    pub csrf_token: String,
    /// A message to show inline if the submit was rejected (taken handle, weak
    /// password); `None` on first render.
    pub error: Option<String>,
    /// The handle the visitor typed, echoed back on a rejected submit.
    pub handle: String,
}
