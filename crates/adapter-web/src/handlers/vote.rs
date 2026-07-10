//! Cast a proposal ballot.

use axum::{
    extract::{Form, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use domain::ProposalId;

use crate::handlers::current_user::current_user;
use crate::handlers::redirect_to_demos::redirect_to_demos;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::vote_form::VoteForm;
use crate::AppState;

pub async fn vote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Form(form): Form<VoteForm>,
) -> Response {
    let lang = resolve_lang(&headers);
    let enhanced = headers.contains_key("x-requested-with");
    let Some(user) = current_user(&state, &headers).await else {
        return if enhanced {
            (StatusCode::UNAUTHORIZED, "sign in").into_response()
        } else {
            render_error(lang, None, "sign in to vote".to_string())
        };
    };

    let pid = ProposalId(id);
    let aye = form.choice == "aye";
    if let Err(e) = state
        .writes
        .cast_vote(pid, user.id, aye, form.signature)
        .await
    {
        return if enhanced {
            (StatusCode::BAD_REQUEST, e.to_string()).into_response()
        } else {
            render_error(lang, Some(user.handle), e.to_string())
        };
    }

    if enhanced {
        // The JS layer updates the tally in place from this JSON.
        let tally = state.services.votes.tally(pid).await.unwrap_or_default();
        return Json(serde_json::json!({ "aye": tally.aye, "nay": tally.nay })).into_response();
    }
    redirect_to_demos(&state, pid).await
}
