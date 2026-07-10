//! Post-upvote + home-feed tests through the `Services` facade.

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
        trials: store.clone(),
        post_votes: store.clone(),
        comment_votes: store.clone(),
        media: store,
        recommender: Arc::new(MemoryRecommender::default()),
        nsfw_scanner: Arc::new(HeuristicNsfwScanner),
        age_verifier: Arc::new(AutoApproveAgeVerifier),
        requires_age_verification: false,
        require_signatures: false,
        clock,
    }
}

async fn text_post(
    svc: &Services,
    author: domain::UserId,
    demos: domain::DemosId,
    title: &str,
) -> domain::PostId {
    svc.create_post(author, demos, title, "b", vec![], vec![])
        .await
        .unwrap()
        .id
}

#[tokio::test]
async fn feed_shows_only_sufficiently_upvoted_joined_posts() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();
    let carol = svc.register_user("carol").await.unwrap();

    // Tiny community (1 voter ⇒ threshold 1). alice founds; bob & carol join.
    let rust = svc
        .found_demos(alice.id, "rust", "Rustaceans")
        .await
        .unwrap();
    svc.join(bob.id, rust.id).await.unwrap();
    svc.join(carol.id, rust.id).await.unwrap();

    let popular = text_post(&svc, alice.id, rust.id, "Popular").await;
    let quiet = text_post(&svc, alice.id, rust.id, "Quiet").await;

    // alice is a member of her own demos and so can vote.
    svc.vote_post(popular, bob.id, Some(true), None)
        .await
        .unwrap();
    svc.vote_post(popular, carol.id, Some(true), None)
        .await
        .unwrap(); // score 2 ≥ 1
                   // `quiet` gets one up and one down ⇒ net 0 < 1.
    svc.vote_post(quiet, bob.id, Some(true), None)
        .await
        .unwrap();
    let score = svc
        .vote_post(quiet, carol.id, Some(false), None)
        .await
        .unwrap();
    assert_eq!(score, 0);

    // alice's feed: only the post clearing the bar, and she joined rust (founder).
    let feed = svc.feed(alice.id).await.unwrap();
    assert_eq!(feed.len(), 1);
    assert_eq!(feed[0].post.id, popular);
    assert_eq!(feed[0].score, 2);
    assert_eq!(feed[0].community_slug, "rust");

    // A user who joined nothing has an empty feed.
    let dave = svc.register_user("dave").await.unwrap();
    assert!(svc.feed(dave.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn top_feed_ranks_all_communities_by_score() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();
    // Two separate communities, both founded by alice (bob joins both).
    let rust = svc
        .found_demos(alice.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let go = svc.found_demos(alice.id, "go", "Gophers").await.unwrap();
    svc.join(bob.id, rust.id).await.unwrap();
    svc.join(bob.id, go.id).await.unwrap();

    let r1 = text_post(&svc, alice.id, rust.id, "rust post").await;
    let g1 = text_post(&svc, alice.id, go.id, "go post").await;

    // go post outscores rust post.
    svc.vote_post(r1, bob.id, Some(true), None).await.unwrap(); // +1
    svc.vote_post(g1, alice.id, Some(true), None).await.unwrap();
    svc.vote_post(g1, bob.id, Some(true), None).await.unwrap(); // +2

    // The top feed spans communities, highest score first, regardless of membership.
    let top = svc.top_posts().await.unwrap();
    assert_eq!(top.len(), 2);
    assert_eq!(top[0].post.id, g1); // higher score leads
    assert_eq!(top[0].score, 2);
    assert_eq!(top[0].community_slug, "go");
    assert_eq!(top[1].post.id, r1);

    // Removed posts are excluded.
    svc.posts.set_removed(r1, true).await.unwrap();
    let top = svc.top_posts().await.unwrap();
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].post.id, g1);
}

#[tokio::test]
async fn votes_toggle_and_clear() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    let rust = svc
        .found_demos(alice.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let p = text_post(&svc, alice.id, rust.id, "Hi").await;

    assert_eq!(
        svc.vote_post(p, alice.id, Some(true), None).await.unwrap(),
        1
    );
    assert_eq!(svc.user_post_vote(p, alice.id).await.unwrap(), Some(true));
    // switch to down
    assert_eq!(
        svc.vote_post(p, alice.id, Some(false), None).await.unwrap(),
        -1
    );
    // clear
    assert_eq!(svc.vote_post(p, alice.id, None, None).await.unwrap(), 0);
    assert_eq!(svc.user_post_vote(p, alice.id).await.unwrap(), None);
}

#[tokio::test]
async fn non_members_cannot_vote() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    let stranger = svc.register_user("stranger").await.unwrap();
    let rust = svc
        .found_demos(alice.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let p = text_post(&svc, alice.id, rust.id, "Hi").await;
    assert!(svc
        .vote_post(p, stranger.id, Some(true), None)
        .await
        .is_err());
}
