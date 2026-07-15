//! Seed a self-contained jury trial you can walk through from every seat.
//!
//! Stands up a throwaway community (`demo-court`) with a small fixed jury, a cast
//! of voter accounts, a rule with a ban term, an offending post by the accused, a
//! report citing that rule, and an open trial. The [`trial_theater`](super::trial_theater)
//! page then lets a dev "sit in" any seat (accused, reporter, juror, bystander)
//! to see and act on the case from that side. Dev-only; see [`super`].

use domain::{JurySizing, ReportReason, ReportTarget, Tier, Timestamp, Trial, UserId};

use crate::AppState;

/// The demo community every seat belongs to. The act-as endpoint scopes session
/// assumption to members of this demos, so it can never point at a real account.
pub(crate) const DEMO_COURT_SLUG: &str = "demo-court";

/// The cast, by handle. `CITIZENS[0]` founds the demos; one citizen also files
/// the report. Eight voters total (seven citizens + the accused) so a strict-
/// minority `Fixed` jury of three can be seated.
pub(crate) const CITIZENS: &[&str] = &["carol", "dave", "erin", "frank", "grace", "heidi", "ivan"];
pub(crate) const ACCUSED: &str = "mallory";
pub(crate) const REPORTER: &str = "carol";

/// Build a fresh case (reusing the community and accounts if already seeded) and
/// return its open trial. Each call files a new post + report + trial, so
/// re-seeding gives a clean case without disturbing the cast.
pub(crate) async fn seed_trial(state: &AppState) -> Result<Trial, String> {
    let svc = &state.services;
    let now = svc.clock.now();

    // The community — founded by the first citizen (who becomes voter #1), reused
    // on re-seed. A small fixed jury keeps the walk-through short: 3 jurors, so 2
    // guilty votes convict.
    let founder = ensure_user(state, CITIZENS[0]).await?;
    let demos = match svc.demoi.by_slug(DEMO_COURT_SLUG).await.map_err(str_err)? {
        Some(d) => d,
        None => svc
            .found_demos(founder, DEMO_COURT_SLUG, "Demo Court")
            .await
            .map_err(|e| e.to_string())?,
    };
    svc.demoi
        .set_jury_sizing(demos.id, JurySizing::Fixed { post: 3, comment: 3 })
        .await
        .map_err(str_err)?;

    // The electorate: every citizen plus the accused, all enfranchised voters. The
    // accused is a voter too, but `open_trial` never seats them on their own jury.
    for handle in CITIZENS {
        ensure_voter(state, handle, demos.id, now).await?;
    }
    let accused = ensure_voter(state, ACCUSED, demos.id, now).await?;

    // A rule carrying a 30-day ban term, so the trial's charge sheet shows a cited
    // rule and the proposed sentence.
    let rule = match svc
        .list_rules(demos.id)
        .await
        .map_err(str_err)?
        .into_iter()
        .find(|r| r.text.starts_with("Be civil"))
    {
        Some(r) => r,
        None => svc
            .rules
            .create(demos.id, "Be civil — no personal attacks", 30, now)
            .await
            .map_err(str_err)?,
    };

    // The offending post, the accusation citing the rule, and the trial.
    let post = svc
        .create_post(
            accused,
            demos.id,
            "Tabs vs spaces",
            "Anyone who indents with spaces is an idiot and shouldn't be allowed to code.",
            vec![],
            vec![],
        )
        .await
        .map_err(|e| e.to_string())?;
    let reporter = svc
        .user_by_handle(REPORTER)
        .await
        .map_err(str_err)?
        .ok_or_else(|| "reporter account missing".to_string())?;
    let report = svc
        .file_report(
            reporter.id,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak {
                rule: Some(rule.id),
            },
            "Personal attack — breaks the ‘be civil’ rule.",
        )
        .await
        .map_err(|e| e.to_string())?;
    svc.open_trial(reporter.id, report.id)
        .await
        .map_err(|e| e.to_string())
}

/// Find the account for `handle`, creating a plain (non-puppet) one if new.
async fn ensure_user(state: &AppState, handle: &str) -> Result<UserId, String> {
    let svc = &state.services;
    match svc.users.by_handle(handle).await.map_err(str_err)? {
        Some(u) => Ok(u.id),
        None => svc.register_user(handle).await.map(|u| u.id).map_err(str_err),
    }
}

/// Ensure `handle` is an enfranchised voter of `demos`, creating the account and
/// membership as needed.
async fn ensure_voter(
    state: &AppState,
    handle: &str,
    demos: domain::DemosId,
    now: Timestamp,
) -> Result<UserId, String> {
    let svc = &state.services;
    let id = ensure_user(state, handle).await?;
    let mut m = match svc.memberships.get(id, demos).await.map_err(str_err)? {
        Some(m) => m,
        None => svc.join(id, demos).await.map_err(|e| e.to_string())?,
    };
    if m.tier != Tier::Voter || m.enfranchised_at.is_none() {
        m.tier = Tier::Voter;
        m.enfranchised_at = Some(now);
        svc.memberships.upsert(m).await.map_err(str_err)?;
    }
    Ok(id)
}

fn str_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
