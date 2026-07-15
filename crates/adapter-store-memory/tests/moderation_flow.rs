//! End-to-end tests for content + moderation: posting, the automatic bot
//! detector, and trial by jury. Wired through `Services` + the in-memory store.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::{Clock, Services};
use domain::{
    DemosId, JurySizing, ProposalKind, ProposalStatus, ReportReason, ReportStatus, ReportTarget,
    Tier, Timestamp, UserId, Verdict, VoteWeighting, WeightingScope,
};

const DAY: i64 = Timestamp::SECONDS_PER_DAY;

/// Register, join, and directly enfranchise a fresh voter — bypassing the
/// Layer-1/2 franchise gates so a test can stand up an electorate large enough
/// to seat a jury without the rate-cap ceremony.
async fn enfranchised(svc: &Services, handle: &str, demos: DemosId, now: Timestamp) -> UserId {
    let u = svc.register_user(handle).await.unwrap();
    svc.join(u.id, demos).await.unwrap();
    let mut m = svc.memberships.get(u.id, demos).await.unwrap().unwrap();
    m.tier = Tier::Voter;
    m.enfranchised_at = Some(now);
    svc.memberships.upsert(m).await.unwrap();
    u.id
}

fn world() -> (Services, Arc<MemoryStore>, Arc<FixedClock>) {
    let store = Arc::new(MemoryStore::new());
    let clock = Arc::new(FixedClock::at(Timestamp(1_000 * DAY)));
    let services = Services {
        users: store.clone(),
        demoi: store.clone(),
        foundings: store.clone(),
        memberships: store.clone(),
        proposals: store.clone(),
        votes: store.clone(),
        rules: store.clone(),
        posts: store.clone(),
        comments: store.clone(),
        reports: store.clone(),
        invites: store.clone(),
        settings: store.clone(),
        sensitive_cases: store.clone(),
        trials: store.clone(),
        notifications: store.clone(),
        trial_comments: store.clone(),
        post_votes: store.clone(),
        comment_votes: store.clone(),
        media: store.clone(),
        recommender: Arc::new(MemoryRecommender::default()),
        nsfw_scanner: Arc::new(HeuristicNsfwScanner),
        age_verifier: Arc::new(AutoApproveAgeVerifier),
        requires_age_verification: false,
        require_signatures: false,
        notifier: Arc::new(adapter_notify::LogNotifier::new()),
        public_base_url: "http://localhost".to_string(),
        invite_token_ttl_days: 7,
        clock: clock.clone(),
    };
    (services, store, clock)
}

/// A passed RuleChange proposal actually creates the rule (effects applied at close).
#[tokio::test]
async fn passed_rule_proposal_creates_the_rule() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    let p = svc
        .open_proposal(
            founder.id,
            demos.id,
            ProposalKind::AddRule {
                text: "Be excellent".into(),
                sanction_days: 0,
            },
        )
        .await
        .unwrap();
    svc.cast_vote(p.id, founder.id, true, None).await.unwrap();
    // A RuleChange votes for 5 days; it can only be closed once that window ends.
    clock.advance_days(5);
    let status = svc.close_proposal(p.id).await.unwrap();
    assert!(
        matches!(status, ProposalStatus::Passed { .. }),
        "got {status:?}"
    );

    let rules = svc.list_rules(demos.id).await.unwrap();
    assert_eq!(rules.len(), 1, "the rule should be in force after close");
    assert_eq!(rules[0].text, "Be excellent");

    // Re-closing an already-decided proposal must be an idempotent no-op — it must
    // not add the rule a second time (the pre-fix duplicate-effect bug).
    let status_again = svc.close_proposal(p.id).await.unwrap();
    assert!(matches!(status_again, ProposalStatus::Passed { .. }));
    assert_eq!(
        svc.list_rules(demos.id).await.unwrap().len(),
        1,
        "re-closing must not duplicate the rule"
    );
}

/// Trial by jury: a reported rule-breaker is convicted by a 2/3 jury, which
/// sanctions them (disqualifying them from the franchise) and removes the post.
#[tokio::test]
async fn guilty_verdict_sanctions_and_removes_content() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let now = clock.now();

    // A 49-voter electorate -> a √n jury of 7 (the conviction bar is 5 guilty).
    for i in 0..48 {
        enfranchised(&svc, &format!("m{i}"), demos.id, now).await;
    }
    let accused = enfranchised(&svc, "troll", demos.id, now).await;

    let post = svc
        .create_post(
            accused,
            demos.id,
            "rule-breaking title",
            "against the rules",
            vec![],
            vec![],
        )
        .await
        .unwrap();

    let report = svc
        .file_report(
            founder.id,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak { rule: None },
            "breaks rule 1",
        )
        .await
        .unwrap();

    let trial = svc.open_trial(founder.id, report.id).await.unwrap();
    assert_eq!(trial.jurors.len(), 7);
    assert!(
        !trial.jurors.contains(&accused),
        "accused never on the jury"
    );

    // Five of seven jurors vote guilty -> 2/3 supermajority -> conviction.
    let mut verdict = Verdict::Pending;
    for juror in trial.jurors.iter().take(5) {
        verdict = svc
            .cast_jury_vote(trial.id, *juror, true, None)
            .await
            .unwrap();
    }
    assert_eq!(verdict, Verdict::Guilty);

    // Consequences applied: post removed, accused sanctioned, report upheld.
    let post = svc.posts.get(post.id).await.unwrap().unwrap();
    assert!(post.removed);
    let m = svc
        .memberships
        .get(accused, demos.id)
        .await
        .unwrap()
        .unwrap();
    assert!(m.is_sanctioned(clock.now()));
    let report = svc.reports.get(report.id).await.unwrap().unwrap();
    assert_eq!(report.status, ReportStatus::Upheld);

    // A sanctioned member can no longer post.
    let err = svc
        .create_post(accused, demos.id, "again", "x", vec![], vec![])
        .await
        .unwrap_err();
    assert!(matches!(err, app::CreatePostError::CanPost(app::CanPostError::Sanctioned)));
}

/// Empanel a 7-seat jury on `report` and convict by a 2/3 supermajority. Returns
/// the time of conviction so the caller can probe the resulting sanction window.
async fn convict_for(
    svc: &Services,
    clock: &FixedClock,
    founder: UserId,
    report_id: domain::ReportId,
) -> Timestamp {
    let trial = svc.open_trial(founder, report_id).await.unwrap();
    assert_eq!(trial.jurors.len(), 7);
    let mut verdict = Verdict::Pending;
    for juror in trial.jurors.iter().take(5) {
        verdict = svc.cast_jury_vote(trial.id, *juror, true, None).await.unwrap();
    }
    assert_eq!(verdict, Verdict::Guilty);
    clock.now()
}

/// A guilty verdict's ban length is the term the voters fixed on the *cited rule*
/// — not a flat maximum. Break a 30-day rule ⇒ a 30-day ban, no more.
#[tokio::test]
async fn conviction_ban_term_comes_from_the_cited_rule() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let now = clock.now();
    for i in 0..47 {
        enfranchised(&svc, &format!("m{i}"), demos.id, now).await;
    }
    let accused = enfranchised(&svc, "troll", demos.id, now).await;

    // A rule carrying a 30-day ban term (as the voters would have set it).
    let rule = svc.rules.create(demos.id, "no flaming", 30, now).await.unwrap();

    let post = svc
        .create_post(accused, demos.id, "t", "b", vec![], vec![])
        .await
        .unwrap();
    let report = svc
        .file_report(
            founder.id,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak { rule: Some(rule.id) },
            "breaks the no-flaming rule",
        )
        .await
        .unwrap();

    let at = convict_for(&svc, &clock, founder.id, report.id).await;
    let m = svc.memberships.get(accused, demos.id).await.unwrap().unwrap();
    // Sanctioned right up to the rule's 30 days...
    assert!(m.is_sanctioned(Timestamp(at.0 + 29 * DAY)));
    // ...and free after — the ban tracked the rule, not the 18-year platform cap.
    assert!(!m.is_sanctioned(Timestamp(at.0 + 31 * DAY)));
}

/// The community's ban ceiling clamps a conviction even when the cited rule asks
/// for longer — the demos-level cap the voters set wins.
#[tokio::test]
async fn conviction_is_clamped_to_the_community_ceiling() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let now = clock.now();
    for i in 0..47 {
        enfranchised(&svc, &format!("m{i}"), demos.id, now).await;
    }
    let accused = enfranchised(&svc, "troll", demos.id, now).await;

    // A harsh 100-day rule, but the community capped every ban at 10 days.
    svc.demoi.set_max_sanction(demos.id, 10).await.unwrap();
    let rule = svc.rules.create(demos.id, "no flaming", 100, now).await.unwrap();

    let post = svc
        .create_post(accused, demos.id, "t", "b", vec![], vec![])
        .await
        .unwrap();
    let report = svc
        .file_report(
            founder.id,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak { rule: Some(rule.id) },
            "",
        )
        .await
        .unwrap();

    let at = convict_for(&svc, &clock, founder.id, report.id).await;
    let m = svc.memberships.get(accused, demos.id).await.unwrap().unwrap();
    assert!(m.is_sanctioned(Timestamp(at.0 + 9 * DAY)), "within the 10-day ceiling");
    assert!(
        !m.is_sanctioned(Timestamp(at.0 + 11 * DAY)),
        "the community ceiling, not the rule's 100 days, governed"
    );
}

/// A minority of guilty votes acquits, leaving the accused untouched.
#[tokio::test]
async fn minority_guilty_acquits() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let now = clock.now();
    for i in 0..48 {
        enfranchised(&svc, &format!("m{i}"), demos.id, now).await;
    }
    let accused = enfranchised(&svc, "accused", demos.id, now).await;
    let post = svc
        .create_post(accused, demos.id, "t", "b", vec![], vec![])
        .await
        .unwrap();
    let report = svc
        .file_report(
            founder.id,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak { rule: None },
            "",
        )
        .await
        .unwrap();
    let trial = svc.open_trial(founder.id, report.id).await.unwrap();

    // 3 not-guilty makes conviction impossible (need 5 guilty of 7) -> acquit.
    let mut verdict = Verdict::Pending;
    for juror in trial.jurors.iter().take(3) {
        verdict = svc
            .cast_jury_vote(trial.id, *juror, false, None)
            .await
            .unwrap();
    }
    assert_eq!(verdict, Verdict::NotGuilty);
    let m = svc
        .memberships
        .get(accused, demos.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!m.is_sanctioned(clock.now()));
    assert!(!svc.posts.get(post.id).await.unwrap().unwrap().removed);
}

/// Flagging the same post a second time for a *different* reason folds the new
/// charge into the original open report instead of opening a parallel case.
#[tokio::test]
async fn a_second_flag_merges_into_the_open_report() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let now = clock.now();
    // Founder + alice + bob = 3 voters, enough to seat a minority jury of 1 once
    // the report goes to trial.
    let alice = enfranchised(&svc, "alice", demos.id, now).await;
    let bob = enfranchised(&svc, "bob", demos.id, now).await;

    let post = svc
        .create_post(founder.id, demos.id, "t", "b", vec![], vec![])
        .await
        .unwrap();

    let first = svc
        .file_report(
            alice,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak { rule: None },
            "off topic",
        )
        .await
        .unwrap();

    // A different member flags the same post for a different reason.
    let merged = svc
        .file_report(
            bob,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::Nsfw,
            "also NSFW",
        )
        .await
        .unwrap();

    // Same case, now carrying both charges with their distinct reporters.
    assert_eq!(merged.id, first.id, "no parallel report was opened");
    let open = svc.reports.list_open(demos.id).await.unwrap();
    assert_eq!(open.len(), 1, "still exactly one open report on the post");
    assert_eq!(open[0].flags.len(), 2);
    assert_eq!(open[0].flags[0].reporter, Some(alice));
    assert_eq!(open[0].flags[1].reporter, Some(bob));

    // Re-filing an identical flag does not inflate the charge sheet.
    svc.file_report(
        alice,
        demos.id,
        ReportTarget::Post(post.id),
        ReportReason::RuleBreak { rule: None },
        "off topic again",
    )
    .await
    .unwrap();
    let open = svc.reports.list_open(demos.id).await.unwrap();
    assert_eq!(open[0].flags.len(), 2, "duplicate flag ignored");

    // Once a trial is empanelled the case is no longer Open, so a fresh flag
    // opens a new report rather than merging.
    svc.open_trial(alice, first.id).await.unwrap();
    let new_report = svc
        .file_report(
            alice,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak { rule: None },
            "still breaking rules",
        )
        .await
        .unwrap();
    assert_ne!(
        new_report.id, first.id,
        "trial charges are fixed; a new case opens"
    );
}

/// Only an unsanctioned voter of the report's community may empanel a jury.
/// Regression: a bare signed-in user (non-member, plain member, or a sanctioned
/// member) could otherwise force any report in any community to trial.
#[tokio::test]
async fn open_trial_requires_a_voter_of_the_reports_community() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let now = clock.now();
    for i in 0..48 {
        enfranchised(&svc, &format!("m{i}"), demos.id, now).await;
    }
    let accused = enfranchised(&svc, "accused", demos.id, now).await;
    let post = svc
        .create_post(accused, demos.id, "t", "b", vec![], vec![])
        .await
        .unwrap();
    let report = svc
        .file_report(
            founder.id,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak { rule: None },
            "",
        )
        .await
        .unwrap();

    // An outsider (no membership at all) cannot open the trial.
    let outsider = svc.register_user("outsider").await.unwrap();
    assert!(matches!(
        svc.open_trial(outsider.id, report.id).await,
        Err(app::OpenTrialError::NotAVoter)
    ));

    // A plain member who is not a voter cannot either.
    let lurker = svc.register_user("lurker").await.unwrap();
    svc.join(lurker.id, demos.id).await.unwrap();
    assert!(matches!(
        svc.open_trial(lurker.id, report.id).await,
        Err(app::OpenTrialError::NotAVoter)
    ));

    // A sanctioned voter is barred as well.
    let mut m = svc
        .memberships
        .get(founder.id, demos.id)
        .await
        .unwrap()
        .unwrap();
    m.sanction_for(clock.now(), 30);
    svc.memberships.upsert(m).await.unwrap();
    assert!(matches!(
        svc.open_trial(founder.id, report.id).await,
        Err(app::OpenTrialError::NotAVoter)
    ));

    // Clear the sanction: a voter in good standing succeeds.
    let mut m = svc
        .memberships
        .get(founder.id, demos.id)
        .await
        .unwrap()
        .unwrap();
    m.clear_sanction();
    svc.memberships.upsert(m).await.unwrap();
    assert!(svc.open_trial(founder.id, report.id).await.is_ok());
}

/// The automatic bot detector files a report (reporter = None) when a fresh
/// account floods a demos with duplicate content — no human action required.
#[tokio::test]
async fn bot_detector_auto_files_report() {
    let (svc, _store, _clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    let bot = svc.register_user("spam9000").await.unwrap();
    svc.join(bot.id, demos.id).await.unwrap();

    // Fresh account (age 0) posting many identical items in the same hour.
    for _ in 0..12 {
        svc.create_post(bot.id, demos.id, "BUY NOW", "spam link", vec![], vec![])
            .await
            .unwrap();
    }

    let open = svc.reports.list_open(demos.id).await.unwrap();
    let auto: Vec<_> = open
        .iter()
        .filter(|r| {
            r.is_automatic()
                && r.flags
                    .iter()
                    .any(|f| matches!(f.reason, ReportReason::Bot))
        })
        .collect();
    assert_eq!(auto.len(), 1, "exactly one auto bot report, de-duplicated");
    assert_eq!(auto[0].target, ReportTarget::User(bot.id));
}

/// The jury that judges a report is a minority of the electorate that shrinks as
/// a share when the demos is large, and is smaller for comments than for posts.
#[tokio::test]
async fn jury_size_scales_with_electorate_and_content_kind() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let now = clock.now();

    // 100 voters (founder + 99) -> √n posts a jury of 10, comments a jury of 5.
    for i in 0..99 {
        enfranchised(&svc, &format!("v{i}"), demos.id, now).await;
    }
    assert_eq!(svc.memberships.voter_count(demos.id).await.unwrap(), 100);

    let post = svc
        .create_post(founder.id, demos.id, "p", "b", vec![], vec![])
        .await
        .unwrap();
    let comment = svc.comment(founder.id, post.id, None, "c").await.unwrap();

    let post_report = svc
        .file_report(
            founder.id,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak { rule: None },
            "",
        )
        .await
        .unwrap();
    let comment_report = svc
        .file_report(
            founder.id,
            demos.id,
            ReportTarget::Comment(comment.id),
            ReportReason::RuleBreak { rule: None },
            "",
        )
        .await
        .unwrap();

    let post_trial = svc.open_trial(founder.id, post_report.id).await.unwrap();
    let comment_trial = svc.open_trial(founder.id, comment_report.id).await.unwrap();

    assert_eq!(post_trial.jurors.len(), 10, "posts: √100 = 10");
    assert_eq!(comment_trial.jurors.len(), 5, "comments: half of that");
    assert!(post_trial.jurors.len() < 50, "still a minority of 100");
}

/// With contribution weighting on, a high-contribution minority can carry a
/// proposal that a one-person-one-vote headcount would sink.
#[tokio::test]
async fn weighting_lets_contribution_carry_a_headcount_minority() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let now = clock.now();

    let whale = enfranchised(&svc, "whale", demos.id, now).await;
    let minnow1 = enfranchised(&svc, "minnow1", demos.id, now).await;
    let minnow2 = enfranchised(&svc, "minnow2", demos.id, now).await;

    // The whale earns contribution -> weight 1 + √100 = 11; everyone else weighs 1.
    svc.record_contribution(whale, demos.id, 100).await.unwrap();
    svc.demoi
        .set_vote_weighting(demos.id, VoteWeighting::ByContribution)
        .await
        .unwrap();
    svc.demoi
        .set_weighting_scope(demos.id, WeightingScope::ProposalsOnly)
        .await
        .unwrap();

    let p = svc
        .open_proposal(
            whale,
            demos.id,
            ProposalKind::RemoveContent {
                target: "post:1".into(),
            },
        )
        .await
        .unwrap();
    svc.cast_vote(p.id, whale, true, None).await.unwrap();
    svc.cast_vote(p.id, minnow1, false, None).await.unwrap();
    svc.cast_vote(p.id, minnow2, false, None).await.unwrap();

    // Headcount: 1 aye vs 2 nay (would fail). Weighted: 11 aye vs 2 nay -> passes.
    clock.advance_days(3); // moderation window
    let status = svc.close_proposal(p.id).await.unwrap();
    assert!(
        matches!(status, ProposalStatus::Passed { .. }),
        "weight should carry it: {status:?}"
    );
}

/// The ban ceiling is governable: a passed `SetMaxSanction` lowers it, and the
/// value is clamped to the 18-year platform cap so no vote can permaban.
#[tokio::test]
async fn a_demos_can_vote_to_lower_its_ban_ceiling() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    // Ask for an absurd term; enactment must clamp it to the platform maximum.
    let p = svc
        .open_proposal(
            founder.id,
            demos.id,
            ProposalKind::SetMaxSanction { days: u32::MAX },
        )
        .await
        .unwrap();
    svc.cast_vote(p.id, founder.id, true, None).await.unwrap();
    clock.advance_days(5); // RuleChange window
    let status = svc.close_proposal(p.id).await.unwrap();
    assert!(matches!(status, ProposalStatus::Passed { .. }), "{status:?}");
    let d = svc.demoi.get(demos.id).await.unwrap().unwrap();
    assert_eq!(d.max_sanction_days, domain::MAX_SANCTION_DAYS, "clamped to 18y");

    // Now lower it to a real, small ceiling.
    let p = svc
        .open_proposal(founder.id, demos.id, ProposalKind::SetMaxSanction { days: 14 })
        .await
        .unwrap();
    svc.cast_vote(p.id, founder.id, true, None).await.unwrap();
    clock.advance_days(5);
    svc.close_proposal(p.id).await.unwrap();
    let d = svc.demoi.get(demos.id).await.unwrap().unwrap();
    assert_eq!(d.max_sanction_days, 14);
    assert_eq!(d.ban_ceiling_days(), 14);
}

/// The jury-sizing policy is itself governable: a passed `SetJurySizing`
/// proposal changes how future reports are juried.
#[tokio::test]
async fn a_demos_can_vote_to_resize_its_juries() {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    let chosen = JurySizing::Fixed {
        post: 3,
        comment: 1,
    };
    let p = svc
        .open_proposal(
            founder.id,
            demos.id,
            ProposalKind::SetJurySizing { sizing: chosen },
        )
        .await
        .unwrap();
    svc.cast_vote(p.id, founder.id, true, None).await.unwrap();
    clock.advance_days(5); // RuleChange window
    let status = svc.close_proposal(p.id).await.unwrap();
    assert!(
        matches!(status, ProposalStatus::Passed { .. }),
        "{status:?}"
    );

    let demos = svc.demoi.get(demos.id).await.unwrap().unwrap();
    assert_eq!(
        demos.jury_sizing, chosen,
        "the vote reshaped the jury policy"
    );
}

/// Stand up an open trial in a small electorate and return (svc, clock, demos,
/// trial, a non-juror voter, the accused). The jury is fixed to 3 so the panel is
/// small and the rest of the electorate are bystander voters.
async fn open_trial_world() -> (
    Services,
    Arc<FixedClock>,
    DemosId,
    domain::Trial,
    UserId,
    UserId,
) {
    let (svc, _store, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    // Fix the jury to 3 so most voters are bystanders, not jurors.
    svc.demoi
        .set_jury_sizing(demos.id, JurySizing::Fixed { post: 3, comment: 3 })
        .await
        .unwrap();
    let now = clock.now();
    for i in 0..8 {
        enfranchised(&svc, &format!("m{i}"), demos.id, now).await;
    }
    let accused = enfranchised(&svc, "troll", demos.id, now).await;
    let post = svc
        .create_post(accused, demos.id, "t", "against the rules", vec![], vec![])
        .await
        .unwrap();
    let report = svc
        .file_report(
            founder.id,
            demos.id,
            ReportTarget::Post(post.id),
            ReportReason::RuleBreak { rule: None },
            "breaks rule 1",
        )
        .await
        .unwrap();
    let trial = svc.open_trial(founder.id, report.id).await.unwrap();
    (svc, clock, demos.id, trial, founder.id, accused)
}

/// Any enfranchised voter — juror or bystander — may comment on a trial, and the
/// gallery reads back oldest-first.
#[tokio::test]
async fn any_voter_can_comment_on_a_trial() {
    let (svc, _clock, _demos, trial, bystander, _accused) = open_trial_world().await;
    // `bystander` (the founder) is a voter but not on the 3-person panel.
    assert!(!trial.jurors.contains(&bystander));
    svc.comment_on_trial(trial.id, bystander, "I was there — this broke rule 1.")
        .await
        .unwrap();
    // A juror can speak too.
    let juror = trial.jurors[0];
    svc.comment_on_trial(trial.id, juror, "Noted, weighing it.")
        .await
        .unwrap();
    let gallery = svc.trial_comments(trial.id).await.unwrap();
    assert_eq!(gallery.len(), 2);
    assert_eq!(gallery[0].author, bystander, "oldest first");
    assert_eq!(gallery[1].author, juror);
}

/// A non-voter (here the sanctioned accused, disqualified from the franchise) may
/// not comment, and an empty comment is refused.
#[tokio::test]
async fn a_non_voter_cannot_comment_and_empty_is_refused() {
    use app::CommentOnTrialError;
    let (svc, _clock, demos, trial, _bystander, _accused) = open_trial_world().await;
    // A brand-new account that never joined is not a voter of this demos.
    let outsider = svc.register_user("outsider").await.unwrap();
    let err = svc
        .comment_on_trial(trial.id, outsider.id, "let me in")
        .await
        .unwrap_err();
    assert!(matches!(err, CommentOnTrialError::NotAVoter));
    // A blank body is rejected even from a bona-fide voter.
    let voter = enfranchised(&svc, "later", demos, svc.clock.now()).await;
    let err = svc
        .comment_on_trial(trial.id, voter, "   ")
        .await
        .unwrap_err();
    assert!(matches!(err, CommentOnTrialError::Empty));
}

/// Count how many of a user's notifications are trial-comment pings (a juror also
/// holds a jury summons from empanelment, so a raw count would conflate the two).
async fn trial_comment_pings(svc: &Services, user: UserId) -> usize {
    svc.notifications(user)
        .await
        .unwrap()
        .iter()
        .filter(|n| matches!(n.kind, domain::NotificationKind::TrialComment { .. }))
        .count()
}

/// A comment on a trial pings the parties to it (the accused, the reporter, the
/// jurors) but never the commenter, and honours the opt-out.
#[tokio::test]
async fn a_trial_comment_notifies_the_parties_but_not_the_author() {
    let (svc, _clock, _demos, trial, reporter, accused) = open_trial_world().await;
    // The reporter may themselves have been drawn onto the jury; pick a juror who
    // is not the comment's author, so their notification count is unambiguous.
    let juror = *trial.jurors.iter().find(|j| **j != reporter).unwrap();
    // The reporter comments. They should not be notified of their own comment;
    // the accused and the jurors should be.
    svc.comment_on_trial(trial.id, reporter, "here's the context")
        .await
        .unwrap();
    assert_eq!(
        trial_comment_pings(&svc, reporter).await,
        0,
        "author is never notified of their own comment"
    );
    assert_eq!(
        trial_comment_pings(&svc, accused).await,
        1,
        "the accused is pinged"
    );
    assert_eq!(trial_comment_pings(&svc, juror).await, 1, "a juror is pinged");

    // The accused opts out; a second comment does not reach them, but the jury
    // still gets it.
    svc.set_alert_prefs(accused, true, true, false).await.unwrap();
    svc.comment_on_trial(trial.id, reporter, "more context")
        .await
        .unwrap();
    assert_eq!(
        trial_comment_pings(&svc, accused).await,
        1,
        "opted-out accused gets no new ping"
    );
    assert_eq!(
        trial_comment_pings(&svc, juror).await,
        2,
        "the juror gets the second ping"
    );
}
