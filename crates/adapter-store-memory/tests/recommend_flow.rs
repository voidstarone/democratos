//! Recommendation use-case tests through the `Services` facade. Pins the
//! contract that the model is built by `refresh_recommendations` (what the web
//! server's background task and the CLI call), and that `recommended_feed` is a
//! pure read of that model.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::Services;
use domain::{Timestamp, UserId};

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
    author: UserId,
    demos: domain::DemosId,
    title: &str,
) -> domain::PostId {
    svc.create_post(author, demos, title, "b", vec![], vec![])
        .await
        .unwrap()
        .id
}

/// Two users with identical taste establish that posts 1, 2 and 4 go together;
/// a third user who liked 1, 2 and 4 should be recommended... nothing new there,
/// so we add a post 5 the cohort also liked. Dave (who hasn't seen 5) should get
/// it recommended, and never the post the cohort downvoted.
#[tokio::test]
async fn recommends_co_liked_posts_after_refresh() {
    let svc = build();
    let alice = svc.register_user("alice").await.unwrap();
    let bob = svc.register_user("bob").await.unwrap();
    let dave = svc.register_user("dave").await.unwrap();
    let d = svc
        .found_demos(alice.id, "rust", "Rustaceans")
        .await
        .unwrap();
    svc.join(bob.id, d.id).await.unwrap();
    svc.join(dave.id, d.id).await.unwrap();

    let p: Vec<_> = {
        let mut v = Vec::new();
        for t in ["async", "borrow", "cooking", "macros", "traits"] {
            v.push(text_post(&svc, alice.id, d.id, t).await);
        }
        v
    };

    // Cohort (alice, bob): like the four technical posts, dislike cooking.
    for u in [alice.id, bob.id] {
        for i in [0, 1, 3, 4] {
            svc.vote_post(p[i], u, Some(true), None).await.unwrap();
        }
        svc.vote_post(p[2], u, Some(false), None).await.unwrap(); // cooking
    }
    // Dave liked three technical posts but never saw #5 (traits).
    for i in [0, 1, 3] {
        svc.vote_post(p[i], dave.id, Some(true), None)
            .await
            .unwrap();
    }

    // Pure read before any refresh: the model is empty, so no recommendations.
    assert!(
        svc.recommend_feed()
            .execute(dave.id, 10)
            .await
            .unwrap()
            .is_empty(),
        "the read use case must not build the model itself"
    );

    // Refresh (what the background task / CLI does) builds the model.
    assert!(
        svc.refresh_recommendations().execute().await.unwrap(),
        "first refresh rebuilds"
    );
    assert!(
        !svc.refresh_recommendations().execute().await.unwrap(),
        "second refresh is a no-op — votes unchanged"
    );

    let recs = svc.recommend_feed().execute(dave.id, 10).await.unwrap();
    let ids: Vec<_> = recs.iter().map(|r| r.post.id).collect();
    assert!(
        ids.contains(&p[4]),
        "recommends the co-liked post #5 (traits): {ids:?}"
    );
    assert!(
        !ids.contains(&p[2]),
        "never recommends the downvoted cooking post"
    );
    for r in &recs {
        assert!(
            r.affinity > 0.0,
            "every recommendation has positive affinity"
        );
        assert!(!ids.contains(&p[0]), "excludes posts dave already voted on");
    }
}
