//! Post a comment to a trial's public gallery discussion.

use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::TrialId;

use crate::handlers::current_user::current_user;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::trial_comment_form::TrialCommentForm;
use crate::AppState;

/// `POST /trial/:id/comments` — a voter adds to a trial's gallery. Only an
/// enfranchised voter of the trial's demos may comment; the service enforces it.
pub async fn comment_on_trial(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Form(form): Form<TrialCommentForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    match state
        .services
        .comment_on_trial(TrialId(id), user.id, &form.body)
        .await
    {
        Ok(_) => Redirect::to(&format!("/trial/{id}")).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
