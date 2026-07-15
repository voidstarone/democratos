//! Personal-blocking tests through the `Services` facade: block/unblock persist,
//! are one-directional and unbounded, and never touch the blocked user's view.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::Services;
use domain::Timestamp;

fn build() -> Services {
    let store = Arc::new(MemoryStore::new());
    let clock = Arc::new(FixedClock::at(Timestamp(1_000)));
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
        media: store,
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

#[tokio::test]
async fn a_block_persists_and_is_one_directional() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();

    svc.block_user(alice.id, bob.id).await.unwrap();

    assert!(svc.is_blocking(alice.id, bob.id).await.unwrap());
    assert_eq!(svc.blocked_by(alice.id).await.unwrap(), vec![bob.id]);
    // The block lives on Alice's record only — Bob sees nothing of it.
    assert!(!svc.is_blocking(bob.id, alice.id).await.unwrap());
    assert!(svc.blocked_by(bob.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn blocking_yourself_is_a_no_op() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    svc.block_user(alice.id, alice.id).await.unwrap();
    assert!(svc.blocked_by(alice.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_repeat_block_is_idempotent_and_unblocking_lifts_it() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();

    svc.block_user(alice.id, bob.id).await.unwrap();
    svc.block_user(alice.id, bob.id).await.unwrap(); // no duplicate
    assert_eq!(svc.blocked_by(alice.id).await.unwrap(), vec![bob.id]);

    svc.unblock_user(alice.id, bob.id).await.unwrap();
    assert!(!svc.is_blocking(alice.id, bob.id).await.unwrap());
    svc.unblock_user(alice.id, bob.id).await.unwrap(); // idempotent
}

#[tokio::test]
async fn there_is_no_cap_on_how_many_you_may_block() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    for i in 0..200 {
        let u = svc.register_user(&format!("spammer{i}")).await.unwrap();
        svc.block_user(alice.id, u.id).await.unwrap();
    }
    assert_eq!(svc.blocked_by(alice.id).await.unwrap().len(), 200);
}
