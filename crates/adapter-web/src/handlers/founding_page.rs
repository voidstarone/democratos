//! A pending founding's petition page handler.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use domain::FoundingId;

use crate::handlers::current_user::current_user;
use crate::handlers::handle_of::handle_of;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::founding_view::FoundingView;
use crate::AppState;

/// A pending founding's page: progress toward the nine required sign-offs, a
/// shareable link, and the sign-off button.
pub async fn founding_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    let Ok(Some(p)) = state.services.founding(FoundingId(id)).await else {
        return render_error(lang, user.map(|u| u.handle), "no such founding".to_string());
    };
    let viewer_id = user.as_ref().map(|u| u.id);
    let is_founder = viewer_id == Some(p.founder);
    let viewer_signed = viewer_id.map(|u| p.is_signed_by(u)).unwrap_or(false);
    render(FoundingView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: user.as_ref().map(|u| u.handle.clone()),
        id: p.id.0,
        slug: p.slug.clone(),
        name: p.name.clone(),
        founder: handle_of(&state, p.founder).await,
        // The founder counts as the first sign-up, so progress shows out of
        // SIGN_OFFS_REQUIRED + 1 (both derived in the domain — never hardcoded).
        signed: p.signed_with_founder(),
        required: p.founding_quorum(),
        is_founder,
        viewer_signed,
        can_sign: viewer_id.is_some() && !is_founder && !viewer_signed,
    })
}
