//! Propose adding a community rule.

use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::ProposalKind;

use crate::handlers::render_error::render_error;
use crate::handlers::require_user_and_demos::require_user_and_demos;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::rule_form::RuleForm;
use crate::AppState;

/// Propose adding a community rule (a RuleChange vote — voters only).
pub async fn propose_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Form(form): Form<RuleForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let (user, demos) = require_user_and_demos!(state, headers, lang, slug);
    match state
        .services
        .open_proposal(user.id, demos.id, ProposalKind::AddRule { text: form.text })
        .await
    {
        Ok(_) => Redirect::to(&format!("/d/{slug}")).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
