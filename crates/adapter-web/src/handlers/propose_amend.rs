//! Propose amending the franchise criteria.

use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::{FranchiseCriteria, ProposalKind};

use crate::handlers::amend_form::AmendForm;
use crate::handlers::render_error::render_error;
use crate::handlers::require_user_and_demos::require_user_and_demos;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

pub async fn propose_amend(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Form(form): Form<AmendForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let (user, demos) = require_user_and_demos!(state, headers, lang, slug);
    let kind = ProposalKind::AmendCriteria {
        proposed: FranchiseCriteria {
            min_account_age_days: form.min_account_age_days,
            min_membership_days: form.min_membership_days,
            min_contribution: form.min_contribution,
        },
    };
    match state.governance.open_proposal(user.id, demos.id, kind).await {
        Ok(_) => Redirect::to(&format!("/d/{slug}")).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
