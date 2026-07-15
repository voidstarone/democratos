//! End-to-end tests for the platform-wide sensitive-content review lane: flagging
//! hides content, a quorum of reviewers classifies it, and the majority tag drives
//! the disposition. Wired through `Services` + the in-memory store.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::Services;
use domain::{PostId, ReportTarget, SensitiveTag, Timestamp};

const DAY: i64 = Timestamp::SECONDS_PER_DAY;

fn world() -> Services {
    let store = Arc::new(MemoryStore::new());
    let clock = Arc::new(FixedClock::at(Timestamp(1_000 * DAY)));
    Services {
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
        clock,
    }
}

/// Stand up a demos with a founder-authored post, and return the post id.
async fn a_post(svc: &Services) -> PostId {
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc.found_demos(founder.id, "rust", "Rustaceans").await.unwrap();
    let post = svc
        .create_post(founder.id, demos.id, "hello", "a body", vec![], vec![])
        .await
        .unwrap();
    post.id
}

/// Register five accounts and opt them all in to reviewing.
async fn five_reviewers(svc: &Services) -> Vec<domain::UserId> {
    let mut ids = Vec::new();
    for i in 0..5 {
        let u = svc.register_user(&format!("reviewer{i}")).await.unwrap();
        svc.set_sensitive_reviewer(u.id, true).await.unwrap();
        ids.push(u.id);
    }
    ids
}

#[tokio::test]
async fn flagging_hides_a_post_pending_review() {
    let svc = world();
    let post = a_post(&svc).await;
    let flagger = svc.register_user("passerby").await.unwrap();

    svc.flag_sensitive(flagger.id, ReportTarget::Post(post), "gross")
        .await
        .unwrap();

    let p = svc.posts.get(post).await.unwrap().unwrap();
    assert!(p.pending_review, "flagging must hide the post pending review");
    assert!(!p.removed);
    // A case is now open.
    assert_eq!(svc.open_case_count().await.unwrap(), 1);
}

#[tokio::test]
async fn non_reviewer_cannot_open_the_queue_or_vote() {
    let svc = world();
    let post = a_post(&svc).await;
    let outsider = svc.register_user("outsider").await.unwrap();
    let case = svc
        .flag_sensitive(outsider.id, ReportTarget::Post(post), "")
        .await
        .unwrap();

    assert!(svc.list_review_queue(outsider.id).await.is_err());
    assert!(svc
        .cast_review_vote(outsider.id, case.id, SensitiveTag::Csam)
        .await
        .is_err());
}

#[tokio::test]
async fn five_porn_votes_age_gate_but_keep_the_post() {
    let svc = world();
    let post = a_post(&svc).await;
    let flagger = svc.register_user("passerby").await.unwrap();
    let case = svc
        .flag_sensitive(flagger.id, ReportTarget::Post(post), "")
        .await
        .unwrap();
    let reviewers = five_reviewers(&svc).await;

    // Four vote before quorum → still open, still hidden.
    for r in &reviewers[..4] {
        svc.cast_review_vote(*r, case.id, SensitiveTag::Porn).await.unwrap();
    }
    assert_eq!(svc.open_case_count().await.unwrap(), 1, "not yet at quorum");
    assert!(svc.posts.get(post).await.unwrap().unwrap().pending_review);

    // Fifth vote reaches quorum; majority is Porn → age-gate and restore.
    svc.cast_review_vote(reviewers[4], case.id, SensitiveTag::Porn).await.unwrap();
    let p = svc.posts.get(post).await.unwrap().unwrap();
    assert!(!p.pending_review, "resolved case un-hides the post");
    assert!(!p.removed, "porn is not removed");
    assert!(p.is_nsfw, "porn is age-gated");
    assert_eq!(svc.open_case_count().await.unwrap(), 0, "case resolved");
}

#[tokio::test]
async fn five_csam_votes_remove_the_post() {
    let svc = world();
    let post = a_post(&svc).await;
    let flagger = svc.register_user("passerby").await.unwrap();
    let case = svc
        .flag_sensitive(flagger.id, ReportTarget::Post(post), "")
        .await
        .unwrap();
    for r in five_reviewers(&svc).await {
        svc.cast_review_vote(r, case.id, SensitiveTag::Csam).await.unwrap();
    }
    let p = svc.posts.get(post).await.unwrap().unwrap();
    assert!(p.removed, "a CSAM majority removes the post");
    assert!(!p.pending_review);
}

#[tokio::test]
async fn a_majority_not_sensitive_restores_the_post() {
    let svc = world();
    let post = a_post(&svc).await;
    let flagger = svc.register_user("passerby").await.unwrap();
    let case = svc
        .flag_sensitive(flagger.id, ReportTarget::Post(post), "")
        .await
        .unwrap();
    let reviewers = five_reviewers(&svc).await;
    // 3 not-sensitive, 2 porn → NotSensitive plurality → restore, no NSFW flag.
    svc.cast_review_vote(reviewers[0], case.id, SensitiveTag::NotSensitive).await.unwrap();
    svc.cast_review_vote(reviewers[1], case.id, SensitiveTag::NotSensitive).await.unwrap();
    svc.cast_review_vote(reviewers[2], case.id, SensitiveTag::NotSensitive).await.unwrap();
    svc.cast_review_vote(reviewers[3], case.id, SensitiveTag::Porn).await.unwrap();
    svc.cast_review_vote(reviewers[4], case.id, SensitiveTag::Porn).await.unwrap();

    let p = svc.posts.get(post).await.unwrap().unwrap();
    assert!(!p.pending_review, "false flag restores the post");
    assert!(!p.removed);
    assert!(!p.is_nsfw, "a false flag does not age-gate");
}
