//! End-to-end secure replication: a write on node A reaches node B's database
//! only through a **signature-verified** change event, tampering is rejected, and
//! re-applying is idempotent. Two real Postgres databases stand in for two nodes.
//!
//! Gated on `TEST_DATABASE_URL` (see `roundtrip.rs`). The test creates a second
//! database next to the configured one for node B.

use adapter_store_postgres::{IncomingChange, OutboxRecord, PostgresStore};
use app::{DemosStore, PostStore, ProposalStore, UserStore, VoteStore};
use domain::{origin_node, NodeId, ProposalKind, Timestamp};
use federation::{ChangeEvent, ChangeOp, NodeKeypair, SignedPart};
use sqlx::Connection;

const NODE_A: u16 = 7;

/// Derive a sibling database URL (same server, different db name).
fn sibling_url(url: &str, db: &str) -> String {
    let (prefix, _old_db) = url.rsplit_once('/').expect("url has a /db");
    format!("{prefix}/{db}")
}

/// Turn one of node A's outbox rows into a signed change event.
fn sign(kp: &NodeKeypair, rec: &OutboxRecord) -> ChangeEvent {
    let op = match rec.op.as_str() {
        "upsert" => ChangeOp::Upsert,
        "delete" => ChangeOp::Delete,
        other => panic!("unexpected op {other}"),
    };
    ChangeEvent::sign(
        kp,
        SignedPart {
            node: 0, // sign() stamps the real node
            epoch: 1,
            seq: rec.seq as u64,
            demos: rec.demos.map(|d| d as u64),
            entity: rec.entity.clone(),
            op,
            payload: rec.payload.clone(),
        },
    )
}

fn op_str(op: ChangeOp) -> &'static str {
    match op {
        ChangeOp::Upsert => "upsert",
        ChangeOp::Delete => "delete",
    }
}

#[tokio::test]
async fn writes_replicate_only_through_verified_events() {
    let Ok(url_a) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };

    // --- stand up node B's database ---
    let db_b = "democratos_repl_b";
    let url_b = sibling_url(&url_a, db_b);
    {
        let mut admin = sqlx::PgConnection::connect(&url_a)
            .await
            .expect("admin conn");
        // DROP/CREATE must run outside a transaction; a single simple query on a
        // bare connection is autocommit, so this works.
        let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {db_b} WITH (FORCE)"))
            .execute(&mut admin)
            .await;
        sqlx::query(&format!("CREATE DATABASE {db_b}"))
            .execute(&mut admin)
            .await
            .expect("create db B");
    }

    // Node A is authoritative; node B replicates it. Each has its own database.
    let node_a = NodeKeypair::generate(NodeId(NODE_A));
    let store_a = PostgresStore::connect(&url_a, NodeId(NODE_A))
        .await
        .expect("connect A");
    let store_b = PostgresStore::connect(&url_b, NodeId(999))
        .await
        .expect("connect B");

    sqlx::query(
        "TRUNCATE users, demoi, memberships, proposals, votes, post_votes, rules, posts, \
         comments, reports, trials, jury_ballots, id_counters, outbox, replication_cursor",
    )
    .execute(store_a.pool())
    .await
    .expect("clean A");

    // --- authoritative writes on A ---
    let alice = UserStore::create(&store_a, "alice", None, None, Timestamp(100))
        .await
        .unwrap();
    let demos = DemosStore::create(&store_a, "rust", "Rustaceans", alice.id, Vec::new(), Timestamp(100))
        .await
        .unwrap();
    let post = PostStore::create(
        &store_a,
        demos.id,
        alice.id,
        "Hello",
        "federated",
        vec![],
        vec![],
        Timestamp(100),
    )
    .await
    .unwrap();

    // A governance proposal and a ballot on it — the ballot is a relational row
    // with no demos_id column, so it exercises review #4's derived scoping.
    let proposal = ProposalStore::create(
        &store_a,
        demos.id,
        alice.id,
        ProposalKind::AddRule {
            text: "be kind".into(),
            sanction_days: 0,
        },
        Timestamp(100),
        Timestamp(9999),
    )
    .await
    .unwrap();
    VoteStore::cast(&store_a, proposal.id, alice.id, true, 1, Timestamp(101))
        .await
        .unwrap();

    // --- #4: the vote event is scoped to its community, not NULL ---
    let vote_rec = store_a
        .outbox_since(0, 1000)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.entity == "votes")
        .expect("a vote event was captured");
    assert_eq!(
        vote_rec.demos,
        Some(demos.id.0 as i64),
        "the ballot event must carry its community, derived via the proposal"
    );

    // --- the feed: sign every outbox row on A ---
    let events: Vec<ChangeEvent> = store_a
        .outbox_since(0, 1000)
        .await
        .unwrap()
        .iter()
        .map(|rec| sign(&node_a, rec))
        .collect();
    assert!(
        events.len() >= 5,
        "user+demos+post+proposal+vote produced events"
    );

    // --- B applies only what verifies against A's public key (#7: one batch) ---
    let a_pub = node_a.public();
    let mut batch = Vec::new();
    for ev in &events {
        let part = ev.verify(&a_pub).expect("A's events verify");
        batch.push(IncomingChange {
            seq: part.seq as i64,
            entity: part.entity.clone(),
            op: op_str(part.op).to_string(),
            payload: part.payload.clone(),
        });
    }
    let applied = store_b
        .apply_batch(NODE_A as i64, &batch)
        .await
        .expect("apply batch");
    assert_eq!(
        applied as usize,
        batch.len(),
        "every fresh event applied once"
    );

    // B now holds A's rows, ids (and their origin node) intact.
    let b_alice = UserStore::get(&store_b, alice.id).await.unwrap().unwrap();
    assert_eq!(b_alice, alice);
    assert_eq!(origin_node(b_alice.id.0), NodeId(NODE_A));
    assert_eq!(
        DemosStore::by_slug(&store_b, "rust")
            .await
            .unwrap()
            .unwrap()
            .id,
        demos.id
    );
    assert_eq!(
        PostStore::get(&store_b, post.id)
            .await
            .unwrap()
            .unwrap()
            .title,
        "Hello"
    );
    // Applying on B did NOT generate a feed on B (no re-publish loop).
    assert!(store_b.outbox_since(0, 10).await.unwrap().is_empty());

    // --- a tampered event is rejected before it can touch B ---
    let recs = store_a.outbox_since(0, 1000).await.unwrap();
    let valid = sign(&node_a, &recs[0]);
    // Attacker rewrites the signed body to rename the account, keeping the old
    // signature. Verification is over the received bytes, so it must fail.
    let forged = ChangeEvent::from_wire(
        valid.body().replace("alice", "attacker"),
        valid.signature().to_string(),
    );
    assert!(
        forged.verify(&a_pub).is_err(),
        "tampered payload must fail verification"
    );
    // Alice on B is unchanged — the forgery never reached apply_change.
    assert_eq!(
        UserStore::by_handle(&store_b, "alice")
            .await
            .unwrap()
            .unwrap()
            .id,
        alice.id
    );
    assert!(UserStore::by_handle(&store_b, "attacker")
        .await
        .unwrap()
        .is_none());

    // --- idempotency: replaying the whole feed applies nothing new ---
    let reapplied = store_b.apply_batch(NODE_A as i64, &batch).await.unwrap();
    assert_eq!(reapplied, 0, "replayed events are all past the cursor");
    assert_eq!(UserStore::list(&store_b).await.unwrap().len(), 1);
    let cursor = store_b.replication_cursor(NODE_A as i64).await.unwrap();
    assert_eq!(cursor, recs.last().unwrap().seq);

    // --- #2: a stale event (seq below the cursor) is skipped, never reverting ---
    let post_payload = recs
        .iter()
        .find(|r| r.entity == "posts")
        .unwrap()
        .payload
        .clone();
    let mut stale = post_payload.clone();
    stale["data"]["title"] = serde_json::json!("STALE — should be ignored");
    let n = store_b
        .apply_batch(
            NODE_A as i64,
            &[IncomingChange {
                seq: 1, // below the cursor
                entity: "posts".into(),
                op: "upsert".into(),
                payload: stale,
            }],
        )
        .await
        .unwrap();
    assert_eq!(n, 0, "stale event must be skipped");
    assert_eq!(
        PostStore::get(&store_b, post.id)
            .await
            .unwrap()
            .unwrap()
            .title,
        "Hello",
        "newer state must not be reverted by a stale replay"
    );
    // The cursor did not move backward.
    assert_eq!(
        store_b.replication_cursor(NODE_A as i64).await.unwrap(),
        cursor
    );

    // --- #6: prune outbox rows every peer has acknowledged ---
    // Simulate a peer that has consumed A's feed up to the first event's seq.
    let ack = recs[0].seq;
    sqlx::query(
        "INSERT INTO replication_cursor (peer_node, last_seq) VALUES (999, $1)
         ON CONFLICT (peer_node) DO UPDATE SET last_seq = EXCLUDED.last_seq",
    )
    .bind(ack)
    .execute(store_a.pool())
    .await
    .unwrap();
    let pruned = store_a.prune_outbox().await.unwrap();
    assert!(pruned >= 1, "acknowledged rows are pruned");
    let remaining = store_a.outbox_since(0, 1000).await.unwrap();
    assert!(
        remaining.iter().all(|r| r.seq > ack),
        "no acknowledged rows remain after pruning"
    );
}

const NODE_C: u16 = 8;

/// A users JSONB payload with the columns the `users` entity spec allows to be
/// populated (`id`, `handle`, `created_at`, `data`).
fn user_payload(id: i64, handle: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "handle": handle,
        "created_at": 100,
        "data": { "id": id, "handle": handle, "created_at": 100 }
    })
}

/// M2 regression: an Overwrite apply whose row collides on a *secondary* UNIQUE
/// index (here `users.handle`) held by a different primary key must NOT abort the
/// whole batch and wedge the per-peer cursor forever. The colliding row is
/// skipped (ON CONFLICT DO NOTHING) while the rest of the batch applies and the
/// cursor advances.
#[tokio::test]
async fn secondary_unique_collision_does_not_wedge_the_cursor() {
    let Ok(url_a) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };

    let db_c = "democratos_repl_c";
    let url_c = sibling_url(&url_a, db_c);
    {
        let mut admin = sqlx::PgConnection::connect(&url_a)
            .await
            .expect("admin conn");
        let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {db_c} WITH (FORCE)"))
            .execute(&mut admin)
            .await;
        sqlx::query(&format!("CREATE DATABASE {db_c}"))
            .execute(&mut admin)
            .await
            .expect("create db C");
    }
    let store = PostgresStore::connect(&url_c, NodeId(999))
        .await
        .expect("connect C");

    // Seed the replica with user id=1, handle "bob".
    let applied = store
        .apply_batch(
            NODE_C as i64,
            &[IncomingChange {
                seq: 1,
                entity: "users".into(),
                op: "upsert".into(),
                payload: user_payload(1, "bob"),
            }],
        )
        .await
        .expect("seed applies");
    assert_eq!(applied, 1);

    // A batch whose middle event (seq 2) reuses handle "bob" under a *different*
    // primary key (id=2) — a secondary-unique collision — followed by a clean row
    // (seq 3, "carol"). Pre-fix this aborted the txn and the cursor never moved.
    let batch = vec![
        IncomingChange {
            seq: 2,
            entity: "users".into(),
            op: "upsert".into(),
            payload: user_payload(2, "bob"),
        },
        IncomingChange {
            seq: 3,
            entity: "users".into(),
            op: "upsert".into(),
            payload: user_payload(3, "carol"),
        },
    ];
    store
        .apply_batch(NODE_C as i64, &batch)
        .await
        .expect("batch must not abort on a secondary-unique collision");

    // The cursor advanced past the whole batch — the puller is not wedged.
    assert_eq!(
        store.replication_cursor(NODE_C as i64).await.unwrap(),
        3,
        "cursor must advance past the colliding batch"
    );
    // "bob" is still the original id=1 row (the colliding id=2 row was skipped).
    assert_eq!(
        UserStore::by_handle(&store, "bob")
            .await
            .unwrap()
            .unwrap()
            .id
            .0,
        1
    );
    // The clean row after the collision still applied.
    assert!(UserStore::by_handle(&store, "carol")
        .await
        .unwrap()
        .is_some());
    // Only bob (id=1) and carol (id=3) exist; the colliding id=2 was not inserted.
    assert_eq!(UserStore::list(&store).await.unwrap().len(), 2);
}
