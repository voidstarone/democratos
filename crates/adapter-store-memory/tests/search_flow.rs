//! Search use-case tests through the `Services` facade. Exercises site-wide vs
//! scoped search, community matching, and the tag filter.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::{SearchScope, Services};
use domain::{Timestamp, SIGN_OFFS_REQUIRED};

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
async fn search_site_wide_scoped_and_by_tag() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    let rust = svc
        .found_demos(alice.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let go = svc.found_demos(alice.id, "go", "Gophers").await.unwrap();

    svc.create_post(
        alice.id,
        rust.id,
        "Borrow checker tips",
        "all about lifetimes",
        vec![],
        vec!["beginner".into()],
    )
    .await
    .unwrap();
    svc.create_post(
        alice.id,
        go.id,
        "Goroutines 101",
        "concurrency basics",
        vec![],
        vec!["beginner".into()],
    )
    .await
    .unwrap();

    // Site-wide: "rust" matches the community by name/slug.
    let r = svc.search("rust", SearchScope::All, None).await.unwrap();
    assert_eq!(r.communities.len(), 1);

    // A title token matches exactly one post.
    let r = svc.search("borrow", SearchScope::All, None).await.unwrap();
    assert_eq!(r.posts.len(), 1);
    assert_eq!(r.posts[0].title, "Borrow checker tips");

    // Scoped search stays within one community (and never returns communities).
    let r = svc
        .search("concurrency", SearchScope::Demos(go.id), None)
        .await
        .unwrap();
    assert_eq!(r.posts.len(), 1);
    assert!(r.communities.is_empty());
    let r = svc
        .search("concurrency", SearchScope::Demos(rust.id), None)
        .await
        .unwrap();
    assert!(r.posts.is_empty());

    // Tag filter, site-wide, with no text query: both posts share the tag.
    let r = svc
        .search("", SearchScope::All, Some("beginner"))
        .await
        .unwrap();
    assert_eq!(r.posts.len(), 2);
    let r = svc
        .search("", SearchScope::All, Some("advanced"))
        .await
        .unwrap();
    assert!(r.posts.is_empty());
}

#[tokio::test]
async fn a_community_is_found_by_its_founding_tags() {
    let svc = build();
    let founder = svc.register_user("founder").await.unwrap();

    // Open a petition carrying topic tags, then gather the co-signers that found it.
    let petition = svc
        .start_founding_tagged(
            founder.id,
            "Rust Systems",
            vec!["rust".into(), "systems".into()],
        )
        .await
        .unwrap();
    let mut born = None;
    for i in 0..SIGN_OFFS_REQUIRED {
        let signer = svc.register_user(&format!("signer{i}")).await.unwrap();
        born = svc.sign_founding(petition.id, signer.id).await.unwrap();
    }
    // The tags captured on the petition land on the demos it becomes.
    let demos = born.expect("the ninth sign-off founds the demos");
    assert_eq!(demos.tags, vec!["rust".to_string(), "systems".to_string()]);

    // The community is now discoverable by either exact tag, site-wide.
    let r = svc.search("", SearchScope::All, Some("rust")).await.unwrap();
    assert_eq!(r.communities.len(), 1);
    assert_eq!(r.communities[0].slug, "rust-systems");
    let r = svc
        .search("", SearchScope::All, Some("systems"))
        .await
        .unwrap();
    assert_eq!(r.communities.len(), 1);

    // The exact-tag needle never prefix-matches: "rus" finds nothing.
    let r = svc.search("", SearchScope::All, Some("rus")).await.unwrap();
    assert!(r.communities.is_empty());
    // A community carries no tags it wasn't founded with.
    let r = svc.search("", SearchScope::All, Some("python")).await.unwrap();
    assert!(r.communities.is_empty());
}
