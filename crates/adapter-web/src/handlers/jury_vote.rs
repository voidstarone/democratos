//! Cast a juror's verdict ballot.

use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::TrialId;

use crate::handlers::current_user::current_user;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::verdict_form::VerdictForm;
use crate::AppState;

pub async fn jury_vote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Form(form): Form<VerdictForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    let guilty = form.verdict == "guilty";
    match state
        .writes
        .cast_jury_vote(TrialId(id), user.id, guilty, form.signature)
        .await
    {
        Ok(_) => Redirect::to(&format!("/trial/{id}")).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
