//! Redirect to the community that owns a proposal.

use axum::response::{IntoResponse, Redirect, Response};
use domain::ProposalId;

use crate::AppState;

pub(crate) async fn redirect_to_demos(state: &AppState, proposal: ProposalId) -> Response {
    if let Ok(Some(p)) = state.services.proposals.get(proposal).await {
        if let Ok(Some(d)) = state.services.demoi.get(p.demos_id).await {
            return Redirect::to(&format!("/d/{}", d.slug)).into_response();
        }
    }
    Redirect::to("/").into_response()
}
