//! Propose changing who may post in a community.

use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::ProposalKind;

use crate::handlers::posting_policy_form::PostingPolicyForm;
use crate::handlers::render_error::render_error;
use crate::handlers::require_user_and_demos::require_user_and_demos;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// Propose changing who may post here (a RuleChange vote — voters only).
pub async fn propose_posting_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Form(form): Form<PostingPolicyForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let (user, demos) = require_user_and_demos!(state, headers, lang, slug);
    let policy = match form.policy.as_str() {
        "open" => domain::PostingPolicy::Open,
        "members" => domain::PostingPolicy::Members,
        "voters" => domain::PostingPolicy::Voters,
        "min" => domain::PostingPolicy::MinContribution(form.threshold.unwrap_or(0).max(0)),
        _ => {
            return render_error(
                lang,
                Some(user.handle),
                "unknown posting policy".to_string(),
            )
        }
    };
    match state
        .services
        .open_proposal(user.id, demos.id, ProposalKind::SetPostingPolicy { policy })
        .await
    {
        Ok(_) => Redirect::to(&format!("/d/{slug}/proposals")).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
