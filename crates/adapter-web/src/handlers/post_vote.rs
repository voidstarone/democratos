//! Up/down vote a post.

use axum::{
    extract::{Form, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use domain::PostId;

use crate::handlers::current_user::current_user;
use crate::handlers::post_vote_form::PostVoteForm;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::safe_referer_back::safe_referer_back;
use crate::AppState;

/// Up/down vote a post. Toggles: clicking the already-active direction clears
/// the vote. Mirrors the proposal `vote` handler's progressive-enhancement
/// contract (JSON on `X-Requested-With`, otherwise redirect back).
pub async fn post_vote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Form(form): Form<PostVoteForm>,
) -> Response {
    let enhanced = headers.contains_key("x-requested-with");
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return if enhanced {
            (StatusCode::UNAUTHORIZED, "sign in").into_response()
        } else {
            render_error(lang, None, "sign in to vote".to_string())
        };
    };

    let pid = PostId(id);
    let clicked = match form.dir.as_str() {
        "up" => Some(true),
        "down" => Some(false),
        _ => None,
    };
    let current = state
        .services
        .user_post_vote(pid, user.id)
        .await
        .unwrap_or(None);
    let target = if current == clicked { None } else { clicked };

    match state
        .writes
        .vote_post(pid, user.id, target, form.signature)
        .await
    {
        Ok(score) => {
            if enhanced {
                let vote = match target {
                    Some(true) => "up",
                    Some(false) => "down",
                    None => "",
                };
                return Json(serde_json::json!({ "score": score, "vote": vote })).into_response();
            }
            let back = safe_referer_back(&headers);
            Redirect::to(&back).into_response()
        }
        Err(e) => {
            if enhanced {
                (StatusCode::BAD_REQUEST, e.to_string()).into_response()
            } else {
                render_error(lang, Some(user.handle), e.to_string())
            }
        }
    }
}
