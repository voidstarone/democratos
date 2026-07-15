//! `GET /dev/trial` — the dev trial-theater page.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use domain::{ReportTarget, TrialId};

use crate::dev::dev_unlocked::dev_unlocked;
use crate::dev::seed_trial::{DEMO_COURT_SLUG, REPORTER};
use crate::dev::trial_query::TrialQuery;
use crate::handlers::cookie_value::cookie_value;
use crate::handlers::handle_of::handle_of;
use crate::handlers::render::render;
use crate::handlers::resolve_lang::resolve_lang;
use crate::views::cast_member::CastMember;
use crate::views::trial_theater_view::TrialTheaterView;
use crate::AppState;

pub async fn trial_theater(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<TrialQuery>,
) -> Response {
    if !dev_unlocked(&state, &headers) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let lang = resolve_lang(&headers);
    let svc = &state.services;
    let current_user = crate::handlers::current_user::current_user(&state, &headers)
        .await
        .map(|u| u.handle);

    // The empty-state page: no community seeded yet.
    let Some(demos) = svc.demoi.by_slug(DEMO_COURT_SLUG).await.ok().flatten() else {
        return render(empty_view(lang, current_user));
    };

    // The trial to show: the one named in the query, else the newest open case.
    let trial = match q.trial {
        Some(id) => svc.trials.get(TrialId(id)).await.ok().flatten(),
        None => {
            let mut open = svc.trials.list_open(demos.id).await.unwrap_or_default();
            open.sort_by(|a, b| b.id.0.cmp(&a.id.0));
            open.into_iter().next()
        }
    };
    let Some(trial) = trial else {
        return render(empty_view(lang, current_user));
    };

    let (guilty, not_guilty) = svc.trials.ballot_tally(trial.id).await.unwrap_or((0, 0));
    let need_guilty = (trial.jury_weight * 2).div_ceil(3);

    // The offending post, from the report behind the trial.
    let post_id = match svc.reports.get(trial.report_id).await.ok().flatten() {
        Some(r) => match r.target {
            ReportTarget::Post(p) => p.0,
            _ => 0,
        },
        None => 0,
    };

    // The current session's account id, to mark the active seat.
    let current_id = cookie_value(&headers, "uid")
        .and_then(|v| state.session.verify(&v))
        .map(|(id, _)| id);

    // The cast: every voter of the demo court, labelled by the seat they occupy.
    let mut cast = Vec::new();
    for m in svc.memberships.members(demos.id).await.unwrap_or_default() {
        let id = m.user_id;
        let handle = handle_of(&state, id).await;
        let is_accused = id == trial.accused;
        let is_juror = trial.jurors.contains(&id);
        let is_reporter = handle == REPORTER;
        let role = if is_accused {
            "Accused".to_string()
        } else if is_juror && is_reporter {
            "Reporter + juror".to_string()
        } else if is_juror {
            "Juror".to_string()
        } else if is_reporter {
            "Reporter".to_string()
        } else {
            "Voter (bystander)".to_string()
        };
        let has_voted = if is_juror {
            svc.trials.has_voted(trial.id, id).await.unwrap_or(false)
        } else {
            false
        };
        cast.push(CastMember {
            id: id.0,
            handle,
            role,
            is_current: current_id == Some(id.0),
            is_juror,
            has_voted,
        });
    }
    // Accused first, then jurors, then everyone else — a natural reading order.
    cast.sort_by_key(|c| {
        if c.role == "Accused" {
            0
        } else if c.is_juror {
            1
        } else {
            2
        }
    });

    render(TrialTheaterView {
        t: lang.strings(),
        lang: lang.code(),
        current_user,
        seeded: true,
        trial_id: trial.id.0,
        accused: handle_of(&state, trial.accused).await,
        verdict: crate::i18n::verdict::verdict(lang, trial.verdict).to_string(),
        guilty,
        not_guilty,
        need_guilty,
        demos_slug: demos.slug,
        post_id,
        cast,
    })
}

fn empty_view(lang: crate::i18n::lang::Lang, current_user: Option<String>) -> TrialTheaterView {
    TrialTheaterView {
        t: lang.strings(),
        lang: lang.code(),
        current_user,
        seeded: false,
        trial_id: 0,
        accused: String::new(),
        verdict: String::new(),
        guilty: 0,
        not_guilty: 0,
        need_guilty: 0,
        demos_slug: DEMO_COURT_SLUG.to_string(),
        post_id: 0,
        cast: Vec::new(),
    }
}
