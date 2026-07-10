//! Integration tests for the Postgres store, run against a real database.
//!
//! Gated on `TEST_DATABASE_URL` — with it unset the test no-ops, so the default
//! `cargo test` on a machine without Postgres stays green. To run it:
//!
//! ```sh
//! docker run -d --name pg -e POSTGRES_PASSWORD=pg -e POSTGRES_USER=app \
//!     -e POSTGRES_DB=democratos -p 55432:5432 postgres:16-alpine
//! TEST_DATABASE_URL=postgres://app:pg@127.0.0.1:55432/democratos \
//!     cargo test -p adapter-store-postgres
//! ```
//!
//! Everything runs in one sequential test: the scenarios share one set of tables,
//! and a global `TRUNCATE` from parallel tests would deadlock on the exclusive
//! lock. One ordered test is both deadlock-free and easier to read.

use adapter_store_postgres::PostgresStore;
use app::StoreError;
use app::{DemosStore, MembershipStore, PostVoteStore, ProposalStore, UserStore, VoteStore};
use domain::{
    origin_node, Membership, NodeId, PostId, ProposalKind, Tally, Tier, Timestamp, UserId,
};

const NODE: u16 = 7;

#[tokio::test]
async fn store_round_trips_against_postgres() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    let store = PostgresStore::connect(&url, NodeId(NODE))
        .await
        .expect("connect + migrate");
    sqlx::query(
        "TRUNCATE users, demoi, memberships, proposals, votes, post_votes, rules, posts, \
         comments, reports, trials, jury_ballots, id_counters",
    )
    .execute(store.pool())
    .await
    .expect("truncate for a clean slate");

    ids_carry_origin_node(&store).await;
    handle_uniqueness_and_lookup(&store).await;
    double_vote_rejected_and_tally_weighted(&store).await;
    voter_count_and_admitted_since(&store).await;
    post_vote_score_toggle_and_clear(&store).await;
}

/// An id minted on this node carries the node in its high bits, and the entity
/// round-trips out of its JSONB document unchanged.
async fn ids_carry_origin_node(store: &PostgresStore) {
    let u = UserStore::create(store, "alice", None, None, Timestamp(100))
        .await
        .unwrap();
    assert_eq!(origin_node(u.id.0), NodeId(NODE));
    let got = UserStore::get(store, u.id).await.unwrap().unwrap();
    assert_eq!(got, u);
}

async fn handle_uniqueness_and_lookup(store: &PostgresStore) {
    let u = UserStore::create(store, "bob", None, None, Timestamp(0))
        .await
        .unwrap();
    let by_handle = UserStore::by_handle(store, "bob").await.unwrap().unwrap();
    assert_eq!(by_handle.id, u.id);
    assert!(UserStore::by_handle(store, "nobody")
        .await
        .unwrap()
        .is_none());
    // The UNIQUE(handle) constraint surfaces as a store error, not a silent dup.
    assert!(UserStore::create(store, "bob", None, None, Timestamp(0))
        .await
        .is_err());
}

/// A repeat ballot is rejected via `ON CONFLICT`, and the tally sums by ballot
/// weight (exercising the `SUM(...) FILTER (WHERE ...)` path).
async fn double_vote_rejected_and_tally_weighted(store: &PostgresStore) {
    let founder = UserStore::create(store, "founder", None, None, Timestamp(0))
        .await
        .unwrap();
    let demos = DemosStore::create(store, "rust", "Rustaceans", founder.id, Timestamp(0))
        .await
        .unwrap();
    let p = ProposalStore::create(
        store,
        demos.id,
        founder.id,
        ProposalKind::AddRule {
            text: "be kind".into(),
        },
        Timestamp(0),
        Timestamp(1000),
    )
    .await
    .unwrap();

    VoteStore::cast(store, p.id, founder.id, true, 3, Timestamp(1))
        .await
        .unwrap();
    let again = VoteStore::cast(store, p.id, founder.id, false, 1, Timestamp(2)).await;
    assert!(matches!(again, Err(StoreError::AlreadyVoted)));

    let carol = UserStore::create(store, "carol", None, None, Timestamp(0))
        .await
        .unwrap();
    VoteStore::cast(store, p.id, carol.id, false, 2, Timestamp(3))
        .await
        .unwrap();

    assert!(VoteStore::has_voted(store, p.id, founder.id).await.unwrap());
    assert_eq!(
        VoteStore::tally(store, p.id).await.unwrap(),
        Tally { aye: 3, nay: 2 }
    );
}

/// `voter_count` / `admitted_since` filter on the lifted `tier` / `enfranchised_at`
/// columns, and `upsert` replaces rather than duplicates.
async fn voter_count_and_admitted_since(store: &PostgresStore) {
    let founder = UserStore::create(store, "gopher", None, None, Timestamp(0))
        .await
        .unwrap();
    let demos = DemosStore::create(store, "go", "Gophers", founder.id, Timestamp(0))
        .await
        .unwrap();

    let mut m = Membership::joined(founder.id, demos.id, Timestamp(0));
    m.tier = Tier::Member; // not a voter → not counted
    store.upsert(m).await.unwrap();

    for (uid, admitted) in [(1_000u64, 100i64), (1_001, 5000)] {
        let mut v = Membership::joined(UserId(uid), demos.id, Timestamp(0));
        v.tier = Tier::Voter;
        v.enfranchised_at = Some(Timestamp(admitted));
        store.upsert(v).await.unwrap();
    }

    assert_eq!(store.voter_count(demos.id).await.unwrap(), 2);
    assert_eq!(
        store
            .admitted_since(demos.id, Timestamp(1000))
            .await
            .unwrap(),
        1
    );

    let mut again = Membership::joined(UserId(1_000), demos.id, Timestamp(0));
    again.tier = Tier::Voter;
    again.enfranchised_at = Some(Timestamp(9999));
    store.upsert(again).await.unwrap();
    assert_eq!(store.voter_count(demos.id).await.unwrap(), 2); // replaced, not duplicated
}

/// Post votes: upsert toggles direction, `None` clears, `score` is the signed sum.
async fn post_vote_score_toggle_and_clear(store: &PostgresStore) {
    let post = PostId(42);
    let (a, b) = (UserId(1), UserId(2));
    PostVoteStore::set(store, post, a, Some(true))
        .await
        .unwrap();
    PostVoteStore::set(store, post, b, Some(true))
        .await
        .unwrap();
    assert_eq!(PostVoteStore::score(store, post).await.unwrap(), 2);
    PostVoteStore::set(store, post, b, Some(false))
        .await
        .unwrap();
    assert_eq!(PostVoteStore::score(store, post).await.unwrap(), 0);
    PostVoteStore::set(store, post, a, None).await.unwrap();
    assert_eq!(PostVoteStore::score(store, post).await.unwrap(), -1);
    assert_eq!(PostVoteStore::get(store, post, a).await.unwrap(), None);
}
