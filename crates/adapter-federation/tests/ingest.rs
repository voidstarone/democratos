//! Closes review #1 end-to-end: replication apply is gated by full authorization.
//! An owner's signed feed applies; a validly-signed feed from a *non-owner*, and
//! a feed from a *fenced* (stale-epoch) owner, are rejected and never touch the
//! replica. Gated on `TEST_DATABASE_URL` (creates a sibling db for the consumer).

use std::sync::Arc;

use adapter_federation::{changes_since, Replicator};
use adapter_store_postgres::PostgresStore;
use app::{DemosStore, PostStore, UserStore};
use domain::{NodeId, Timestamp};
use federation::{
    AuthError, ChangeEvent, ChangeOp, InMemoryRegistry, NodeKeypair, OwnershipRegistry, SignedPart,
};
use sqlx::Connection;

const OWNER_NODE: u16 = 7;
const IMPOSTOR_NODE: u16 = 8;

fn sibling_url(url: &str, db: &str) -> String {
    let (prefix, _) = url.rsplit_once('/').unwrap();
    format!("{prefix}/{db}")
}

#[tokio::test]
async fn apply_is_gated_by_ownership_and_epoch() {
    let Ok(url_a) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    let db_b = "democratos_ingest_b";
    let url_b = sibling_url(&url_a, db_b);
    {
        let mut admin = sqlx::PgConnection::connect(&url_a).await.unwrap();
        let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {db_b} WITH (FORCE)"))
            .execute(&mut admin)
            .await;
        sqlx::query(&format!("CREATE DATABASE {db_b}"))
            .execute(&mut admin)
            .await
            .unwrap();
    }

    let store_a = PostgresStore::connect(&url_a, NodeId(OWNER_NODE))
        .await
        .unwrap();
    let store_b = Arc::new(PostgresStore::connect(&url_b, NodeId(999)).await.unwrap());
    sqlx::query(
        "TRUNCATE users, demoi, memberships, proposals, votes, post_votes, rules, posts, \
         comments, reports, trials, jury_ballots, id_counters, outbox, replication_cursor",
    )
    .execute(store_a.pool())
    .await
    .unwrap();

    // Control plane: the owner node's key is published and it owns the community.
    let owner_kp = NodeKeypair::generate(NodeId(OWNER_NODE));
    let registry = Arc::new(InMemoryRegistry::new());
    registry
        .publish_key(NodeId(OWNER_NODE), &owner_kp.public().to_hex())
        .await
        .unwrap();

    // Authoritative writes on the owner.
    let alice = UserStore::create(&store_a, "alice", None, None, Timestamp(1))
        .await
        .unwrap();
    let demos = DemosStore::create(&store_a, "rust", "Rustaceans", alice.id, Timestamp(1))
        .await
        .unwrap();
    let post = PostStore::create(
        &store_a,
        demos.id,
        alice.id,
        "Hello",
        "hi",
        vec![],
        vec![],
        Timestamp(1),
    )
    .await
    .unwrap();

    // The owner now actually owns the community in the registry (epoch 1).
    registry
        .claim(demos.id.0, NodeId(OWNER_NODE))
        .await
        .unwrap();

    let replicator = Replicator::new(
        store_b.clone(),
        registry.clone() as Arc<dyn OwnershipRegistry>,
    );

    // --- owner's signed feed applies ---
    let feed = changes_since(&store_a, &owner_kp, registry.as_ref(), 0, 1000)
        .await
        .unwrap();
    let out = replicator.ingest(OWNER_NODE as i64, &feed).await.unwrap();
    assert_eq!(out.applied as usize, feed.len(), "owner feed fully applied");
    assert!(out.rejected.is_empty());
    assert_eq!(
        PostStore::get(&*store_b, post.id)
            .await
            .unwrap()
            .unwrap()
            .title,
        "Hello"
    );

    // Helper: the raw outbox rows, so we can re-sign them as an attacker would.
    let recs = store_a.outbox_since(0, 1000).await.unwrap();
    let post_rec = recs.iter().find(|r| r.entity == "posts").unwrap();
    let hijacked_payload = {
        let mut p = post_rec.payload.clone();
        p["data"]["title"] = serde_json::json!("HIJACKED");
        p
    };
    let part = |node_epoch: u64| SignedPart {
        node: 0,
        epoch: node_epoch,
        seq: post_rec.seq as u64,
        demos: Some(demos.id.0),
        entity: "posts".into(),
        op: ChangeOp::Upsert,
        payload: hijacked_payload.clone(),
    };

    // --- a non-owner's validly-signed feed is rejected (NotOwner) ---
    let impostor = NodeKeypair::generate(NodeId(IMPOSTOR_NODE));
    registry
        .publish_key(NodeId(IMPOSTOR_NODE), &impostor.public().to_hex())
        .await
        .unwrap();
    let forged = ChangeEvent::sign(&impostor, part(1));
    let out = replicator
        .ingest(IMPOSTOR_NODE as i64, std::slice::from_ref(&forged))
        .await
        .unwrap();
    assert_eq!(out.applied, 0, "non-owner event must not apply");
    assert_eq!(out.rejected, vec![AuthError::NotOwner]);

    // --- a fenced (stale-epoch) owner event is rejected (StaleEpoch) ---
    // Simulate a handoff that bumped the epoch to 2, then the old owner speaks at 1.
    registry
        .release(demos.id.0, NodeId(OWNER_NODE))
        .await
        .unwrap();
    registry
        .claim(demos.id.0, NodeId(OWNER_NODE))
        .await
        .unwrap(); // epoch → 2
    let stale = ChangeEvent::sign(&owner_kp, part(1)); // old epoch
    let out = replicator
        .ingest(OWNER_NODE as i64, std::slice::from_ref(&stale))
        .await
        .unwrap();
    assert_eq!(out.applied, 0, "stale-epoch event must not apply");
    assert_eq!(out.rejected, vec![AuthError::StaleEpoch]);

    // The post on the replica was never hijacked through any rejected path.
    assert_eq!(
        PostStore::get(&*store_b, post.id)
            .await
            .unwrap()
            .unwrap()
            .title,
        "Hello"
    );
}

/// A transient rejection must not lose data. When a peer pulls a newly-created
/// community's events **before its owner has claimed it** (the boot / found
/// window), those demos-scoped events read as `Unowned` and are rejected — but
/// the cursor must NOT skip past them, so once ownership settles a later pull
/// applies them. Regression for the ordered-log data-loss bug the federation
/// stress harness surfaced. Gated on `TEST_DATABASE_URL`.
#[tokio::test]
async fn a_transiently_rejected_event_is_retried_not_skipped() {
    let Ok(url_a) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    // Own owner + replica DBs, so this test is isolated from the one above (both
    // share the same test-database process and run in parallel).
    let db_a = "democratos_ingest_retry_a";
    let db_b = "democratos_ingest_retry_b";
    let url_owner = sibling_url(&url_a, db_a);
    let url_b = sibling_url(&url_a, db_b);
    {
        let mut admin = sqlx::PgConnection::connect(&url_a).await.unwrap();
        for db in [db_a, db_b] {
            let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {db} WITH (FORCE)"))
                .execute(&mut admin)
                .await;
            sqlx::query(&format!("CREATE DATABASE {db}"))
                .execute(&mut admin)
                .await
                .unwrap();
        }
    }

    let store_a = PostgresStore::connect(&url_owner, NodeId(OWNER_NODE))
        .await
        .unwrap();
    let store_b = Arc::new(PostgresStore::connect(&url_b, NodeId(998)).await.unwrap());

    let owner_kp = NodeKeypair::generate(NodeId(OWNER_NODE));
    let registry = Arc::new(InMemoryRegistry::new());
    registry
        .publish_key(NodeId(OWNER_NODE), &owner_kp.public().to_hex())
        .await
        .unwrap();

    // Writes happen (a user, a community, a post) BEFORE the owner claims it —
    // exactly the window between founding and the ownership heartbeat.
    let alice = UserStore::create(&store_a, "alice", None, None, Timestamp(1))
        .await
        .unwrap();
    let demos = DemosStore::create(&store_a, "rust", "Rustaceans", alice.id, Timestamp(1))
        .await
        .unwrap();
    let post = PostStore::create(
        &store_a,
        demos.id,
        alice.id,
        "Hello",
        "hi",
        vec![],
        vec![],
        Timestamp(1),
    )
    .await
    .unwrap();

    let replicator = Replicator::new(
        store_b.clone(),
        registry.clone() as Arc<dyn OwnershipRegistry>,
    );

    // First pull, still unowned: the leading unscoped `users` row applies, but the
    // demos-scoped `demoi`/`posts` rows are rejected — and the cursor stops there.
    let cursor0 = replicator.cursor(OWNER_NODE as i64).await.unwrap();
    let feed = changes_since(&store_a, &owner_kp, registry.as_ref(), cursor0, 1000)
        .await
        .unwrap();
    let out = replicator.ingest(OWNER_NODE as i64, &feed).await.unwrap();
    assert!(
        !out.rejected.is_empty(),
        "the unowned demos events must be rejected"
    );
    assert!(
        PostStore::get(&*store_b, post.id).await.unwrap().is_none(),
        "the post must NOT be on the replica yet"
    );
    let cursor1 = replicator.cursor(OWNER_NODE as i64).await.unwrap();
    assert!(
        cursor1 < demos_seq(&store_a, demos.id.0).await,
        "cursor must not have advanced past the rejected demos event"
    );

    // Ownership settles (the heartbeat claims it).
    registry
        .claim(demos.id.0, NodeId(OWNER_NODE))
        .await
        .unwrap();

    // A later pull resumes from the cursor and now applies the previously-rejected
    // events — nothing was lost.
    let cursor = replicator.cursor(OWNER_NODE as i64).await.unwrap();
    let feed = changes_since(&store_a, &owner_kp, registry.as_ref(), cursor, 1000)
        .await
        .unwrap();
    let out = replicator.ingest(OWNER_NODE as i64, &feed).await.unwrap();
    assert!(
        out.rejected.is_empty(),
        "with an owner, the retry authorizes cleanly"
    );
    assert_eq!(
        PostStore::get(&*store_b, post.id)
            .await
            .unwrap()
            .unwrap()
            .title,
        "Hello",
        "the post is on the replica after the retry — no data lost"
    );
}

/// A **permanently** unauthorizable event (a dethroned owner's fenced event
/// after a rehoming) must be skipped so the cursor steps past it — otherwise it
/// stalls all later replication from that peer. Regression for the post-failover
/// stall the chaos harness surfaced. Gated on `TEST_DATABASE_URL`.
#[tokio::test]
async fn a_permanently_rejected_event_is_skipped_not_stalled() {
    let Ok(url_a) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    let db_a = "democratos_ingest_skip_a";
    let db_b = "democratos_ingest_skip_b";
    let url_owner = sibling_url(&url_a, db_a);
    let url_b = sibling_url(&url_a, db_b);
    {
        let mut admin = sqlx::PgConnection::connect(&url_a).await.unwrap();
        for db in [db_a, db_b] {
            let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {db} WITH (FORCE)"))
                .execute(&mut admin)
                .await;
            sqlx::query(&format!("CREATE DATABASE {db}"))
                .execute(&mut admin)
                .await
                .unwrap();
        }
    }

    let store_a = PostgresStore::connect(&url_owner, NodeId(OWNER_NODE))
        .await
        .unwrap();
    let store_b = Arc::new(PostgresStore::connect(&url_b, NodeId(997)).await.unwrap());

    let owner_kp = NodeKeypair::generate(NodeId(OWNER_NODE));
    let impostor = NodeKeypair::generate(NodeId(IMPOSTOR_NODE));
    let registry = Arc::new(InMemoryRegistry::new());
    registry
        .publish_key(NodeId(OWNER_NODE), &owner_kp.public().to_hex())
        .await
        .unwrap();
    registry
        .publish_key(NodeId(IMPOSTOR_NODE), &impostor.public().to_hex())
        .await
        .unwrap();

    let alice = UserStore::create(&store_a, "alice", None, None, Timestamp(1))
        .await
        .unwrap();
    let demos = DemosStore::create(&store_a, "rust", "Rustaceans", alice.id, Timestamp(1))
        .await
        .unwrap();
    let p1 = PostStore::create(
        &store_a,
        demos.id,
        alice.id,
        "first",
        "b",
        vec![],
        vec![],
        Timestamp(1),
    )
    .await
    .unwrap();
    let p2 = PostStore::create(
        &store_a,
        demos.id,
        alice.id,
        "second",
        "b",
        vec![],
        vec![],
        Timestamp(1),
    )
    .await
    .unwrap();
    let p3 = PostStore::create(
        &store_a,
        demos.id,
        alice.id,
        "third",
        "b",
        vec![],
        vec![],
        Timestamp(1),
    )
    .await
    .unwrap();
    registry
        .claim(demos.id.0, NodeId(OWNER_NODE))
        .await
        .unwrap();

    // Build the real feed, then re-sign p2's event with the IMPOSTOR key so it
    // reads as `NotOwner` — a stand-in for a fenced event mid-stream.
    let mut feed = changes_since(&store_a, &owner_kp, registry.as_ref(), 0, 1000)
        .await
        .unwrap();
    let recs = store_a.outbox_since(0, 1000).await.unwrap();
    let p2_rec = recs
        .iter()
        .find(|r| r.entity == "posts" && r.payload["id"].as_i64() == Some(p2.id.0 as i64))
        .unwrap();
    let forged = ChangeEvent::sign(
        &impostor,
        SignedPart {
            node: 0,
            epoch: 1,
            seq: p2_rec.seq as u64,
            demos: Some(demos.id.0),
            entity: "posts".into(),
            op: ChangeOp::Upsert,
            payload: p2_rec.payload.clone(),
        },
    );
    for ev in feed.iter_mut() {
        if ev.peek().unwrap().seq == p2_rec.seq as u64 {
            *ev = forged.clone();
        }
    }

    let replicator = Replicator::new(
        store_b.clone(),
        registry.clone() as Arc<dyn OwnershipRegistry>,
    );
    let out = replicator.ingest(OWNER_NODE as i64, &feed).await.unwrap();

    // p1 and p3 applied; the forged p2 was skipped (not applied) and the cursor
    // stepped over it — so replication did NOT stall on it.
    assert!(
        !out.rejected.is_empty(),
        "the forged event must be reported rejected"
    );
    assert_eq!(
        PostStore::get(&*store_b, p1.id)
            .await
            .unwrap()
            .unwrap()
            .title,
        "first"
    );
    assert_eq!(
        PostStore::get(&*store_b, p3.id)
            .await
            .unwrap()
            .unwrap()
            .title,
        "third"
    );
    assert!(
        PostStore::get(&*store_b, p2.id).await.unwrap().is_none(),
        "forged p2 not applied"
    );
    // The cursor stepped past the skipped event all the way to the feed head, so
    // a later pull is not blocked by it (no stall).
    let head = store_a
        .outbox_since(0, 10_000)
        .await
        .unwrap()
        .last()
        .unwrap()
        .seq;
    assert_eq!(
        replicator.cursor(OWNER_NODE as i64).await.unwrap(),
        head,
        "cursor reached the feed head, stepping over the fenced event"
    );
}

/// The outbox seq of the `demoi` row for `demos`, for the cursor assertion above.
async fn demos_seq(store: &PostgresStore, demos: u64) -> i64 {
    store
        .outbox_since(0, 10_000)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.entity == "demoi" && r.demos == Some(demos as i64))
        .expect("a demoi outbox row")
        .seq
}
