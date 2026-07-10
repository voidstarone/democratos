//! The open-reports queue page handler (plus its reason/summary helpers).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use domain::{ReportReason, ReportStatus, ReportTarget};

use crate::handlers::current_user::current_user;
use crate::handlers::handle_of::handle_of;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::i18n::lang::Lang;
use crate::views::report_row::ReportRow;
use crate::views::reports_view::ReportsView;
use crate::AppState;

pub async fn reports_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    let Ok(Some(demos)) = state.services.demoi.by_slug(&slug).await else {
        return render_error(lang, user.map(|u| u.handle), "no such demos".to_string());
    };

    let mut reports = Vec::new();
    if let Ok(open) = state.services.reports.list_open(demos.id).await {
        for r in open {
            let reporter = match r.founding().reporter {
                Some(u) => handle_of(&state, u).await,
                None => lang.strings().auto_detector.to_string(),
            };
            let on_trial = match r.status {
                ReportStatus::OnTrial(t) => Some(t.0),
                _ => None,
            };
            // One pill per distinct reason on the case, in the order they were filed.
            let mut reasons: Vec<String> = Vec::new();
            for flag in &r.flags {
                let label = report_reason(lang, &flag.reason).to_string();
                if !reasons.contains(&label) {
                    reasons.push(label);
                }
            }
            reports.push(ReportRow {
                id: r.id.0,
                summary: report_summary(&state, &r).await,
                reasons,
                reporter,
                on_trial,
            });
        }
    }

    render(ReportsView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: user.map(|u| u.handle),
        slug: demos.slug,
        reports,
    })
}

fn report_reason(lang: Lang, reason: &ReportReason) -> &'static str {
    match (lang, reason) {
        (Lang::En, ReportReason::Bot) => "bot",
        (Lang::En, ReportReason::RuleBreak { .. }) => "rule-break",
        (Lang::En, ReportReason::Nsfw) => "NSFW",
        (Lang::Es, ReportReason::Bot) => "bot",
        (Lang::Es, ReportReason::RuleBreak { .. }) => "infracción",
        (Lang::Es, ReportReason::Nsfw) => "NSFW",
    }
}

async fn report_summary(state: &AppState, report: &domain::Report) -> String {
    let note = &report.founding().note;
    match report.target {
        ReportTarget::Post(p) => {
            let title = state
                .services
                .posts
                .get(p)
                .await
                .ok()
                .flatten()
                .map(|p| p.title)
                .unwrap_or_else(|| format!("#{}", p.0));
            format!("post: {title} — {note}")
        }
        ReportTarget::Comment(c) => format!("comment #{} — {note}", c.0),
        ReportTarget::User(u) => {
            format!("user {} — {note}", handle_of(state, u).await)
        }
    }
}
