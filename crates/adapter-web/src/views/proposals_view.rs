use askama::Template;

use crate::i18n::strings::Strings;
use crate::views::proposal_view::ProposalView;

/// The dedicated governance page for one community: its proposals plus the forms
/// a voter uses to open new ones. Split off the community page so posts stay the
/// hero there.
#[derive(Template)]
#[template(path = "proposals.html")]
pub struct ProposalsView {
    pub t: Strings,
    pub lang: &'static str,
    pub current_user: Option<String>,
    pub slug: String,
    pub phase: String,
    pub viewer_is_voter: bool,
    /// True once the demos has left Seed and may amend its constitution.
    pub can_amend: bool,
    pub criteria_age: i64,
    pub criteria_member: i64,
    pub criteria_contrib: i64,
    /// Human label of the current posting policy (e.g. "members", "popularity ≥ 5").
    pub posting_policy: String,
    /// The current policy as a form key (`open`/`members`/`voters`/`min`) so the
    /// picker preselects it.
    pub posting_policy_kind: String,
    /// The current MinContribution threshold (0 when the policy isn't threshold-based).
    pub posting_policy_threshold: i64,
    /// The community's current ban ceiling in days — the max a rule term may set
    /// and the default shown in the ceiling form.
    pub max_sanction_days: u32,
    /// The 18-year platform cap — the upper bound on the ceiling itself.
    pub platform_max_sanction_days: u32,
    pub proposals: Vec<ProposalView>,
}
