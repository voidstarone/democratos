//! Flag a post as sensitive, hiding it pending platform-wide review.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::{PostId, ReportTarget};

use crate::handlers::current_user::current_user;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

/// Flag a post as sensitive. Any signed-in user may flag; the post is hidden from
/// normal feeds immediately and a platform-wide review case opens (or the flag
/// merges into the open one). Redirects home — the post is no longer viewable to
/// the flagger.
pub async fn flag_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    match state
        .services
        .flag_sensitive(user.id, ReportTarget::Post(PostId(id)), "user-flagged")
        .await
    {
        Ok(_) => Redirect::to("/").into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
