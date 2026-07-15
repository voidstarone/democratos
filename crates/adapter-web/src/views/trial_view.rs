use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::charge_view::ChargeView;
use crate::views::trial_comment_view::TrialCommentView;

#[derive(Template)]
#[template(path = "trial.html")]
pub struct TrialView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub id: u64,
    pub accused: String,
    pub jurors: Vec<String>,
    pub verdict: String,
    pub open: bool,
    pub guilty: u64,
    pub not_guilty: u64,
    pub viewer_is_juror: bool,
    /// The charge sheet from the report behind this trial — the context jurors
    /// weigh. Empty only if the report has since vanished.
    pub charges: Vec<ChargeView>,
    /// The ban term (days) a guilty verdict would impose, derived from the cited
    /// rule(s). `0` when it can't be resolved (report gone).
    pub proposed_days: u32,
    /// The trial's public gallery discussion, oldest first.
    pub comments: Vec<TrialCommentView>,
    /// Whether the viewer is an enfranchised voter of this demos, and so may post
    /// to the gallery. Drives whether the comment form is shown.
    pub viewer_can_comment: bool,
}
