//! The community governance page handler (plus its view builder).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use domain::{Phase, ProposalStatus, User};

use crate::handlers::current_user::current_user;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::i18n::lang::Lang;
use crate::views::proposal_view::ProposalView;
use crate::views::proposals_view::ProposalsView;
use crate::AppState;

/// The community's governance page: its proposals and the forms to open new
/// ones. Reached from a button next to Open Reports.
pub async fn proposals_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    match build_proposals_view(&state, lang, &slug, user.as_ref()).await {
        Ok(view) => render(view),
        Err(e) => render_error(lang, user.map(|u| u.handle), e.to_string()),
    }
}

async fn build_proposals_view(
    state: &AppState,
    lang: Lang,
    slug: &str,
    viewer: Option<&User>,
) -> app::Result<ProposalsView> {
    let demos = state
        .services
        .demoi
        .by_slug(slug)
        .await?
        .ok_or(app::StoreError::NotFound)?;
    let voters = state.services.memberships.voter_count(demos.id).await?;
    let phase = Phase::from_voter_count(voters);

    let viewer_is_voter = match viewer {
        Some(u) => state
            .services
            .memberships
            .get(u.id, demos.id)
            .await?
            .map(|m| m.is_voter())
            .unwrap_or(false),
        None => false,
    };

    let mut proposals = Vec::new();
    for p in state.services.proposals.list(demos.id).await? {
        let tally = state.services.votes.tally(p.id).await.unwrap_or_default();
        proposals.push(ProposalView {
            id: p.id.0,
            title: crate::i18n::proposal_title::proposal_title(lang, &p.kind),
            class: crate::i18n::class::class(lang, &p.kind).to_string(),
            status: crate::i18n::status::status(lang, &p.status).to_string(),
            open: matches!(p.status, ProposalStatus::Open),
            aye: tally.aye,
            nay: tally.nay,
        });
    }
    let (posting_policy_kind, posting_policy_threshold) = match demos.posting_policy {
        domain::PostingPolicy::Open => ("open", 0),
        domain::PostingPolicy::Members => ("members", 0),
        domain::PostingPolicy::Voters => ("voters", 0),
        domain::PostingPolicy::MinContribution(n) => ("min", n),
    };

    Ok(ProposalsView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: viewer.map(|u| u.handle.clone()),
        slug: demos.slug.clone(),
        phase: crate::i18n::phase::phase(lang, phase).to_string(),
        viewer_is_voter,
        can_amend: phase != Phase::Seed,
        criteria_age: demos.criteria.min_account_age_days,
        criteria_member: demos.criteria.min_membership_days,
        criteria_contrib: demos.criteria.min_contribution,
        posting_policy: crate::i18n::posting_policy_label::posting_policy_label(
            lang,
            demos.posting_policy,
        ),
        posting_policy_kind: posting_policy_kind.to_string(),
        posting_policy_threshold,
        max_sanction_days: demos.max_sanction_days,
        platform_max_sanction_days: domain::MAX_SANCTION_DAYS,
        proposals,
    })
}
