//! Add a comment to a post.

use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};
use domain::{CommentId, PostId};

use crate::handlers::comment_form::CommentForm;
use crate::handlers::current_user::current_user;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

pub async fn add_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Form(form): Form<CommentForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    let parent = form.parent.map(CommentId);
    match state
        .services
        .comment(user.id, PostId(id), parent, &form.body)
        .await
    {
        Ok(_) => Redirect::to(&format!("/post/{id}")).into_response(),
        Err(e) => render_error(lang, Some(user.handle), e.to_string()),
    }
}
