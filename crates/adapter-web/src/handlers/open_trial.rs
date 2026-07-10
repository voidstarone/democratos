//! Open a jury trial from a report.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::ReportId;

use crate::handlers::current_user::current_user;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

pub async fn open_trial(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    // Authorization (member/voter of the report's community) is enforced in the
    // use-case; the handler only needs to supply the acting user.
    match state.services.open_trial(user.id, ReportId(id)).await {
        Ok(t) => Redirect::to(&format!("/trial/{}", t.id.0)).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
