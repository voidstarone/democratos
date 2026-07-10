use askama::Template;

use crate::i18n::strings::Strings;

/// The petition page for one pending founding: its progress toward quorum, a
/// shareable link, and the sign-off action.
#[derive(Template)]
#[template(path = "founding.html")]
pub struct FoundingView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub id: u64,
    pub slug: String,
    pub name: String,
    pub founder: String,
    pub signed: usize,
    pub required: usize,
    /// The viewer started this founding.
    pub is_founder: bool,
    /// The viewer has already signed off.
    pub viewer_signed: bool,
    /// The viewer may sign off now (signed in, not the founder, not yet signed).
    pub can_sign: bool,
}
