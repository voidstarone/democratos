//! Founding-petition tests: a demos is only born once nine other people sign
//! off, and the founder plus co-signers land in it with the right roles.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::{Services, SignFoundingError, StartFoundingError, StoreError};
use domain::{Timestamp, UserId, SIGN_OFFS_REQUIRED};

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

/// Register `n` extra sign-off accounts named signer0..signer{n-1}.
async fn signers(svc: &Services, n: usize) -> Vec<UserId> {
    let mut ids = Vec::new();
    for i in 0..n {
        ids.push(svc.register_user(&format!("signer{i}")).await.unwrap().id);
    }
    ids
}

#[tokio::test]
async fn demos_is_born_only_on_the_ninth_signoff() {
    let svc = build();
    let founder = svc.register_user("founder").await.unwrap();

    // Opening a petition does not create a community.
    let petition = svc.start_founding(founder.id, "Rust Fans").await.unwrap();
    assert_eq!(petition.slug, "rust-fans", "slug is derived from the name");
    assert!(svc.demoi.by_slug("rust-fans").await.unwrap().is_none());
    assert_eq!(svc.pending_foundings().await.unwrap().len(), 1);

    let signers = signers(&svc, SIGN_OFFS_REQUIRED).await;

    // The first eight sign-offs keep it pending.
    for &s in &signers[..SIGN_OFFS_REQUIRED - 1] {
        assert!(svc.sign_founding(petition.id, s).await.unwrap().is_none());
    }
    assert!(svc.demoi.by_slug("rust-fans").await.unwrap().is_none());

    // The ninth founds the demos.
    let demos = svc
        .sign_founding(petition.id, signers[SIGN_OFFS_REQUIRED - 1])
        .await
        .unwrap()
        .expect("ninth sign-off founds the demos");
    assert_eq!(demos.slug, "rust-fans");
    assert_eq!(demos.founder_id, founder.id);

    // The petition is cleared once founded.
    assert!(svc.pending_foundings().await.unwrap().is_empty());
    assert!(svc.founding(petition.id).await.unwrap().is_none());

    // All ten (founder + nine co-signers) are founding voters.
    assert_eq!(
        svc.memberships.voter_count(demos.id).await.unwrap(),
        1 + SIGN_OFFS_REQUIRED as u64
    );
    let founder_m = svc
        .memberships
        .get(founder.id, demos.id)
        .await
        .unwrap()
        .unwrap();
    assert!(founder_m.is_voter());
    for &s in &signers {
        let m = svc.memberships.get(s, demos.id).await.unwrap().unwrap();
        assert!(m.is_voter(), "every co-signer becomes a founding voter");
    }
}

#[tokio::test]
async fn signoffs_are_idempotent_and_exclude_the_founder() {
    let svc = build();
    let founder = svc.register_user("founder").await.unwrap();
    let ally = svc.register_user("ally").await.unwrap();
    let petition = svc.start_founding(founder.id, "Gophers").await.unwrap();

    // The founder cannot pad their own quorum.
    assert!(matches!(
        svc.sign_founding(petition.id, founder.id).await,
        Err(SignFoundingError::Rejected(_))
    ));

    // A repeated sign-off by the same user counts once.
    svc.sign_founding(petition.id, ally.id).await.unwrap();
    svc.sign_founding(petition.id, ally.id).await.unwrap();
    let p = svc.founding(petition.id).await.unwrap().unwrap();
    assert_eq!(p.sign_offs, vec![ally.id]);
}

#[tokio::test]
async fn cannot_start_a_founding_that_collides() {
    let svc = build();
    let a = svc.register_user("a").await.unwrap();
    let b = svc.register_user("b").await.unwrap();

    // A blank/punctuation-only name has no slug and is rejected.
    assert!(matches!(
        svc.start_founding(a.id, "!!!").await,
        Err(StartFoundingError::Rejected(_))
    ));

    // Two open petitions can't claim the same derived slug.
    svc.start_founding(a.id, "Rust Fans").await.unwrap();
    assert!(matches!(
        svc.start_founding(b.id, "rust-fans").await,
        Err(StartFoundingError::Store(StoreError::AlreadyExists))
    ));
}
