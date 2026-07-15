//! The dev trial-theater page: walk a jury trial from every seat.

use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::cast_member::CastMember;

/// The dev-only trial theater at `/dev/trial`. Shows the seeded case and its
/// cast, each with an "act as" control, so one browser can play out a trial from
/// the accused's, reporter's, jurors', and a bystander's point of view.
#[derive(Template)]
#[template(path = "trial_theater.html")]
pub struct TrialTheaterView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    /// False before anything is seeded — the page then shows just the seed button.
    pub seeded: bool,
    pub trial_id: u64,
    pub accused: String,
    /// Localized verdict label (Pending / Guilty / Not guilty).
    pub verdict: String,
    pub guilty: u64,
    pub not_guilty: u64,
    /// Guilty votes needed to convict (2/3 supermajority of the panel).
    pub need_guilty: u64,
    pub demos_slug: String,
    /// The offending post, for the "view the post" link.
    pub post_id: u64,
    pub cast: Vec<CastMember>,
}
