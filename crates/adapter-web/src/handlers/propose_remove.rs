//! Propose removing a piece of content.

use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::ProposalKind;

use crate::handlers::remove_form::RemoveForm;
use crate::handlers::render_error::render_error;
use crate::handlers::require_user_and_demos::require_user_and_demos;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

pub async fn propose_remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Form(form): Form<RemoveForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let (user, demos) = require_user_and_demos!(state, headers, lang, slug);
    let kind = ProposalKind::RemoveContent {
        target: form.target,
    };
    match state.services.open_proposal(user.id, demos.id, kind).await {
        Ok(_) => Redirect::to(&format!("/d/{slug}")).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
