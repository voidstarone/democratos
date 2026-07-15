//! Propose changing the community's ban ceiling.

use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::ProposalKind;

use crate::handlers::max_sanction_form::MaxSanctionForm;
use crate::handlers::render_error::render_error;
use crate::handlers::require_user_and_demos::require_user_and_demos;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// Propose setting the community's ceiling on any single ban (a RuleChange vote —
/// voters only, part of the rulebook). Enactment clamps the value to the 18-year
/// platform cap, so this can only ever *lower* the ceiling below that maximum —
/// never permaban.
pub async fn propose_max_sanction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Form(form): Form<MaxSanctionForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let (user, demos) = require_user_and_demos!(state, headers, lang, slug);
    match state
        .services
        .open_proposal(
            user.id,
            demos.id,
            ProposalKind::SetMaxSanction { days: form.days },
        )
        .await
    {
        Ok(_) => Redirect::to(&format!("/d/{slug}/proposals")).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
