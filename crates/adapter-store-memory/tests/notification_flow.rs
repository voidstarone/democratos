//! In-app notifications through the `Services` facade: a member is pinged when
//! named (`@handle`) in content and when summoned to a jury, each opt-in.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::{Clock, Services};
use domain::{DemosId, NotificationKind, ReportReason, ReportTarget, Tier, Timestamp, UserId};

const DAY: i64 = Timestamp::SECONDS_PER_DAY;

fn world() -> (Services, Arc<FixedClock>) {
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
        media: store,
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
    (services, clock)
}

async fn enfranchised(svc: &Services, handle: &str, demos: DemosId, now: Timestamp) -> UserId {
    let u = svc.register_user(handle).await.unwrap();
    svc.join(u.id, demos).await.unwrap();
    let mut m = svc.memberships.get(u.id, demos).await.unwrap().unwrap();
    m.tier = Tier::Voter;
    m.enfranchised_at = Some(now);
    svc.memberships.upsert(m).await.unwrap();
    u.id
}

#[tokio::test]
async fn a_mention_in_a_post_pings_the_named_member() {
    let (svc, _clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();
    let demos = svc.found_demos(founder.id, "rust", "Rustaceans").await.unwrap();

    let post = svc
        .create_post(founder.id, demos.id, "hello", "hey @bob and @nobody", vec![], vec![])
        .await
        .unwrap();

    // Bob is pinged; the unknown handle is silently dropped.
    let notes = svc.notifications(bob.id).await.unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(svc.unread_notification_count(bob.id).await.unwrap(), 1);
    match notes[0].kind {
        NotificationKind::Mention { post: p, comment, by } => {
            assert_eq!(p, post.id);
            assert_eq!(comment, None);
            assert_eq!(by, founder.id);
        }
        _ => panic!("expected a mention"),
    }
}

#[tokio::test]
async fn a_mention_in_a_comment_links_the_comment() {
    let (svc, _clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();
    let demos = svc.found_demos(founder.id, "rust", "Rustaceans").await.unwrap();
    let post = svc
        .create_post(founder.id, demos.id, "t", "b", vec![], vec![])
        .await
        .unwrap();

    let comment = svc.comment(founder.id, post.id, None, "ping @bob").await.unwrap();
    let notes = svc.notifications(bob.id).await.unwrap();
    assert_eq!(notes.len(), 1);
    match notes[0].kind {
        NotificationKind::Mention { comment: c, .. } => assert_eq!(c, Some(comment.id)),
        _ => panic!("expected a comment mention"),
    }
}

#[tokio::test]
async fn you_are_not_pinged_for_mentioning_yourself() {
    let (svc, _clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc.found_demos(founder.id, "rust", "Rustaceans").await.unwrap();
    svc.create_post(founder.id, demos.id, "hi", "note to self @founder", vec![], vec![])
        .await
        .unwrap();
    assert!(svc.notifications(founder.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn opting_out_of_mention_alerts_suppresses_them() {
    let (svc, _clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();
    // Bob keeps jury alerts but turns mention alerts off.
    svc.set_alert_prefs(bob.id, false, true, true).await.unwrap();
    let demos = svc.found_demos(founder.id, "rust", "Rustaceans").await.unwrap();
    svc.create_post(founder.id, demos.id, "t", "hey @bob", vec![], vec![])
        .await
        .unwrap();
    assert!(svc.notifications(bob.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn empanelling_a_jury_summons_each_juror() {
    let (svc, clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc.found_demos(founder.id, "rust", "Rustaceans").await.unwrap();
    let now = clock.now();
    for i in 0..47 {
        enfranchised(&svc, &format!("m{i}"), demos.id, now).await;
    }
    let accused = enfranchised(&svc, "troll", demos.id, now).await;
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
    // Every empanelled juror got exactly one jury-summons notification.
    for juror in &trial.jurors {
        let notes = svc.notifications(*juror).await.unwrap();
        let summons: Vec<_> = notes
            .iter()
            .filter(|n| matches!(n.kind, NotificationKind::JurySummons { trial: t, .. } if t == trial.id))
            .collect();
        assert_eq!(summons.len(), 1, "juror {juror:?} summoned once");
    }
    // The accused is never on the jury, so never summoned.
    assert!(svc
        .notifications(accused)
        .await
        .unwrap()
        .iter()
        .all(|n| !matches!(n.kind, NotificationKind::JurySummons { .. })));
}

#[tokio::test]
async fn opening_the_list_marks_notifications_seen() {
    let (svc, _clock) = world();
    let founder = svc.register_user("founder").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();
    let demos = svc.found_demos(founder.id, "rust", "Rustaceans").await.unwrap();
    svc.create_post(founder.id, demos.id, "t", "@bob @bob hi", vec![], vec![])
        .await
        .unwrap();

    // A handle repeated in one body pings once.
    assert_eq!(svc.unread_notification_count(bob.id).await.unwrap(), 1);
    svc.mark_notifications_seen(bob.id).await.unwrap();
    assert_eq!(svc.unread_notification_count(bob.id).await.unwrap(), 0);
    // The notification is still listed — only its unread state cleared.
    assert_eq!(svc.notifications(bob.id).await.unwrap().len(), 1);
}
