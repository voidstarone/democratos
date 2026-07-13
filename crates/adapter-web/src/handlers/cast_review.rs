//! Record a reviewer's classification on a case.

use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::{SensitiveCaseId, SensitiveTag};

use crate::handlers::classify_form::ClassifyForm;
use crate::handlers::current_user::current_user;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// Cast the signed-in reviewer's classification on a case, then return to the
/// queue. Once at least [`REVIEW_QUORUM`](domain::REVIEW_QUORUM) reviewers have
/// classified it, the service layer resolves the case and applies the disposition.
pub async fn cast_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Form(form): Form<ClassifyForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    let Some(tag) = SensitiveTag::from_slug(form.tag.trim()) else {
        return render_error(lang, Some(user.handle), "unknown classification".to_string());
    };
    match state
        .services
        .cast_review_vote(user.id, SensitiveCaseId(id), tag)
        .await
    {
        Ok(_) => Redirect::to("/review").into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
