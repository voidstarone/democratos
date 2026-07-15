//! Close and tally a proposal (plus its authorization helper).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use domain::{ProposalId, UserId};

use crate::handlers::current_user::current_user;
use crate::handlers::redirect_to_demos::redirect_to_demos;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

pub async fn close_proposal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Response {
    let lang = resolve_lang(&headers);
    let pid = ProposalId(id);

    // Closing tallies the proposal *now* (the domain ignores `closes_at`) and can
    // apply a rule/constitutional change — a governance action, not a public one.
    // Gate it: the caller must be signed in and a voter of the proposal's demos,
    // or an anonymous request could close any proposal by id to freeze a tally at
    // a chosen moment. Checked before any state change, unlike the read-only page
    // handlers.
    let Some(user) = current_user(&state, &headers).await else {
        return render_error(lang, None, "sign in first".to_string());
    };
    if !is_voter_of_proposal(&state, user.id, pid).await {
        return render_error(
            lang,
            Some(user.handle),
            "only a voter of this community may close its proposals".to_string(),
        );
    }

    if let Err(e) = state.governance.close_proposal(pid).await {
        return render_error(lang, Some(user.handle), e.to_string());
    }
    redirect_to_demos(&state, pid).await
}

/// Whether `user` is an enfranchised voter of the demos that owns `proposal`.
/// A missing proposal, membership, or non-voter standing all deny.
async fn is_voter_of_proposal(state: &AppState, user: UserId, proposal: ProposalId) -> bool {
    let Ok(Some(p)) = state.services.proposals.get(proposal).await else {
        return false;
    };
    matches!(
        state.services.memberships.get(user, p.demos_id).await,
        Ok(Some(m)) if m.is_voter()
    )
}
