//! `GET /d/:slug/trials` — a community's public case log.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use domain::Verdict;

use crate::handlers::current_user::current_user;
use crate::handlers::handle_of::handle_of;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::case_log_view::CaseLogView;
use crate::views::case_row::CaseRow;
use crate::AppState;

/// Every trial a community has held — ongoing and past — newest first. Trials are
/// public record, so this is reachable signed out; it's linked only quietly from
/// the community page, never the main nav.
pub async fn case_log_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Response {
    let lang = resolve_lang(&headers);
    let viewer = current_user(&state, &headers).await.map(|u| u.handle);

    let demos = match state.services.demoi.by_slug(&slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return render_error(lang, viewer, "no such community".to_string()),
        Err(e) => return render_error(lang, viewer, e.to_string()),
    };

    let trials = state
        .services
        .trials
        .list_for_demos(demos.id)
        .await
        .unwrap_or_default();
    let mut cases = Vec::new();
    for tr in trials {
        cases.push(CaseRow {
            trial_id: tr.id.0,
            accused: handle_of(&state, tr.accused).await,
            verdict: crate::i18n::verdict::verdict(lang, tr.verdict).to_string(),
            open: tr.verdict == Verdict::Pending,
        });
    }

    render(CaseLogView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: viewer,
        slug: demos.slug,
        name: demos.name,
        cases,
    })
}
