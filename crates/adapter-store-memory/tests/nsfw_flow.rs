//! NSFW detection + voting (forbid policy → jury) + the age-verification gate,
//! exercised through the `Services` facade.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::Services;
use domain::{ProposalKind, ReportReason, Timestamp, Visibility};

fn build(requires_age_verification: bool) -> (Services, Arc<FixedClock>) {
    let store = Arc::new(MemoryStore::new());
    let clock = Arc::new(FixedClock::at(Timestamp(1_000)));
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
        post_votes: store.clone(),
        comment_votes: store.clone(),
        media: store.clone(),
        recommender: Arc::new(MemoryRecommender::default()),
        nsfw_scanner: Arc::new(HeuristicNsfwScanner),
        age_verifier: Arc::new(AutoApproveAgeVerifier),
        requires_age_verification,
        require_signatures: false,
        notifier: Arc::new(adapter_notify::LogNotifier::new()),
        public_base_url: "http://localhost".to_string(),
        invite_token_ttl_days: 7,
        clock: clock.clone(),
    };
    (services, clock)
}

#[tokio::test]
async fn detects_flags_and_allows_but_gates_by_default() {
    let (svc, _clock) = build(false); // age verification off
    let alice = svc.register_user("alice").await.unwrap();
    let d = svc.found_demos(alice.id, "art", "Art").await.unwrap();

    // An explicit post is flagged; a clean one is not.
    let dirty = svc
        .create_post(alice.id, d.id, "explicit nude study", "x", vec![], vec![])
        .await
        .unwrap();
    assert!(dirty.is_nsfw, "explicit text is flagged");

    let clean = svc
        .create_post(alice.id, d.id, "a sunset", "lovely", vec![], vec![])
        .await
        .unwrap();
    assert!(!clean.is_nsfw);

    // Allowed-but-gated: no report filed when the community permits NSFW.
    assert!(svc.reports.list_open(d.id).await.unwrap().is_empty());

    // Toggle off → flagged content is Blurred (revealable), clean is Visible.
    assert_eq!(
        svc.post_visibility(&dirty, alice.id).await.unwrap(),
        Visibility::Blurred
    );
    assert_eq!(
        svc.post_visibility(&clean, alice.id).await.unwrap(),
        Visibility::Visible
    );
}

#[tokio::test]
async fn forbidding_community_auto_reports_for_a_jury() {
    let (svc, clock) = build(false);
    let alice = svc.register_user("alice").await.unwrap();
    let d = svc
        .found_demos(alice.id, "sfw", "Strictly SFW")
        .await
        .unwrap();

    // The founder (sole voter, Seed phase) votes to forbid NSFW; close applies it.
    let p = svc
        .open_proposal(
            alice.id,
            d.id,
            ProposalKind::SetNsfwPolicy { allows_nsfw: false },
        )
        .await
        .unwrap();
    svc.cast_vote(p.id, alice.id, true, None).await.unwrap();
    // SetNsfwPolicy is a RuleChange (5-day window); close only after it elapses.
    clock.advance_days(5);
    svc.close_proposal(p.id).await.unwrap();
    assert!(!svc.demoi.get(d.id).await.unwrap().unwrap().allows_nsfw);

    // Now an explicit post is both flagged AND auto-reported (machine accuses).
    let post = svc
        .create_post(alice.id, d.id, "xxx hardcore", "x", vec![], vec![])
        .await
        .unwrap();
    assert!(post.is_nsfw);
    let reports = svc.reports.list_open(d.id).await.unwrap();
    assert_eq!(reports.len(), 1);
    assert!(reports[0]
        .flags
        .iter()
        .any(|f| matches!(f.reason, ReportReason::Nsfw)));
    assert!(
        reports[0].is_automatic(),
        "filed by the machine, not a member"
    );
}

#[tokio::test]
async fn age_gate_withholds_until_verified() {
    let (svc, _clock) = build(true); // age verification ON (e.g. UK)
    let alice = svc.register_user("alice").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();
    let d = svc.found_demos(alice.id, "art", "Art").await.unwrap();
    let post = svc
        .create_post(alice.id, d.id, "explicit nude study", "x", vec![], vec![])
        .await
        .unwrap();
    assert!(post.is_nsfw);

    // Unverified viewer: gated (withheld).
    assert_eq!(
        svc.post_visibility(&post, bob.id).await.unwrap(),
        Visibility::Gated
    );

    // After verification: blurred (may reveal).
    assert!(svc.verify_age(bob.id).await.unwrap());
    assert_eq!(
        svc.post_visibility(&post, bob.id).await.unwrap(),
        Visibility::Blurred
    );
}
