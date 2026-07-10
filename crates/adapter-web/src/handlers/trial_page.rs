//! The jury-trial page handler (plus its view builder).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use domain::{TrialId, User, Verdict};

use crate::handlers::current_user::current_user;
use crate::handlers::handle_of::handle_of;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::i18n::lang::Lang;
use crate::views::trial_view::TrialView;
use crate::AppState;

pub async fn trial_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    match build_trial_view(&state, lang, TrialId(id), user.as_ref()).await {
        Ok(view) => render(view),
        Err(e) => render_error(lang, user.map(|u| u.handle), e.to_string()),
    }
}

async fn build_trial_view(
    state: &AppState,
    lang: Lang,
    trial_id: TrialId,
    viewer: Option<&User>,
) -> app::Result<TrialView> {
    let trial = state
        .services
        .trials
        .get(trial_id)
        .await?
        .ok_or(app::StoreError::NotFound)?;
    let (guilty, not_guilty) = state.services.trials.ballot_tally(trial_id).await?;

    let mut jurors = Vec::new();
    for j in &trial.jurors {
        jurors.push(handle_of(state, *j).await);
    }

    Ok(TrialView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: viewer.map(|u| u.handle.clone()),
        id: trial.id.0,
        accused: handle_of(state, trial.accused).await,
        jurors,
        verdict: crate::i18n::verdict::verdict(lang, trial.verdict).to_string(),
        open: trial.verdict == Verdict::Pending,
        guilty: guilty as u64,
        not_guilty: not_guilty as u64,
        viewer_is_juror: viewer.map(|u| trial.is_juror(u.id)).unwrap_or(false),
    })
}
