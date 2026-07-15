//! The jury-trial page handler (plus its view builder).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use domain::{ReportReason, TrialId, User, Verdict};

use crate::handlers::current_user::current_user;
use crate::handlers::handle_of::handle_of;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::i18n::lang::Lang;
use crate::views::charge_view::ChargeView;
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

    // The charge sheet: every flag on the report behind this trial, with its cited
    // rule spelled out — so jurors judge with the full context, not a bare id.
    let mut charges = Vec::new();
    if let Some(report) = state.services.reports.get(trial.report_id).await? {
        for flag in &report.flags {
            let rule_text = match flag.reason {
                ReportReason::RuleBreak { rule: Some(id) } => {
                    state.services.rules.get(id).await?.map(|r| r.text)
                }
                _ => None,
            };
            let by = match flag.reporter {
                Some(uid) => handle_of(state, uid).await,
                None => lang.strings().report_automatic.to_string(),
            };
            charges.push(ChargeView {
                reason: reason_label(lang, &flag.reason).to_string(),
                by,
                note: flag.note.clone(),
                rule_text,
            });
        }
    }
    let proposed_days = state
        .services
        .proposed_ban_term(trial.report_id)
        .await?
        .unwrap_or(0);

    // The public gallery discussion, oldest first.
    let mut comments = Vec::new();
    for c in state.moderation.trial_comments(trial.id).await? {
        comments.push(crate::views::trial_comment_view::TrialCommentView {
            by: handle_of(state, c.author).await,
            body: c.body,
        });
    }

    // A comment form is offered only to an enfranchised voter of this demos — the
    // same right the service enforces on submit.
    let viewer_can_comment = match viewer {
        Some(u) => state
            .services
            .memberships
            .get(u.id, trial.demos_id)
            .await?
            .map(|m| m.is_franchised(state.services.clock.now()))
            .unwrap_or(false),
        None => false,
    };

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
        charges,
        proposed_days,
        comments,
        viewer_can_comment,
    })
}

/// Localized short label for a flag's reason, shown on the charge sheet.
fn reason_label(lang: Lang, reason: &ReportReason) -> &'static str {
    match (lang, reason) {
        (Lang::En, ReportReason::Bot) => "bot",
        (Lang::En, ReportReason::RuleBreak { .. }) => "rule-break",
        (Lang::En, ReportReason::Nsfw) => "NSFW",
        (Lang::Es, ReportReason::Bot) => "bot",
        (Lang::Es, ReportReason::RuleBreak { .. }) => "infracción",
        (Lang::Es, ReportReason::Nsfw) => "NSFW",
    }
}
