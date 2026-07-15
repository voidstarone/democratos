//! End-to-end HTTP transport: node A serves its signed feed over a real axum
//! server, node B pulls it over HTTP and applies the authorized events, and the
//! node-to-node bearer token is enforced. Gated on `TEST_DATABASE_URL`.

use std::sync::Arc;
use std::time::Duration;

use adapter_federation::{feed_router, FeedClient, FeedState, Peer, Replicator};
use adapter_store_postgres::PostgresStore;
use app::{DemosStore, PostStore, UserStore};
use domain::{NodeId, Timestamp};
use federation::{InMemoryRegistry, NodeKeypair, OwnershipRegistry};
use sqlx::Connection;

const OWNER_NODE: u16 = 7;
const TOKEN: &str = "cluster-secret";

fn sibling_url(url: &str, db: &str) -> String {
    let (prefix, _) = url.rsplit_once('/').unwrap();
    format!("{prefix}/{db}")
}

#[tokio::test]
async fn feed_is_served_and_pulled_over_http() {
    let Ok(url_a) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    let db_b = "democratos_transport_b";
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

    let store_a = Arc::new(
        PostgresStore::connect(&url_a, NodeId(OWNER_NODE))
            .await
            .unwrap(),
    );
    let store_b = Arc::new(PostgresStore::connect(&url_b, NodeId(999)).await.unwrap());
    sqlx::query("TRUNCATE users, demoi, posts, id_counters, outbox, replication_cursor")
        .execute(store_a.pool())
        .await
        .unwrap();

    let owner_kp = Arc::new(NodeKeypair::generate(NodeId(OWNER_NODE)));
    let registry = Arc::new(InMemoryRegistry::new());
    registry
        .publish_key(NodeId(OWNER_NODE), &owner_kp.public().to_hex())
        .await
        .unwrap();

    // Authoritative writes on A.
    let alice = UserStore::create(&*store_a, "alice", None, None, Timestamp(1))
        .await
        .unwrap();
    let demos = DemosStore::create(&*store_a, "rust", "Rustaceans", alice.id, Vec::new(), Timestamp(1))
        .await
        .unwrap();
    let post = PostStore::create(
        &*store_a,
        demos.id,
        alice.id,
        "Hello over HTTP",
        "hi",
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

    // --- start A's feed server on an ephemeral port ---
    let state = FeedState {
        store: store_a.clone(),
        keypair: owner_kp.clone(),
        registry: registry.clone(),
        token: Some(TOKEN.to_string()),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, feed_router(state)).await.unwrap();
    });
    // Give the server a moment to accept connections.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let base = format!("http://{addr}");

    // --- wrong token is refused (401) ---
    let bad = FeedClient::new(&base, Some("nope".into()));
    assert!(
        bad.changes_since(0, 100).await.is_err(),
        "a bad token must be rejected"
    );

    // --- node B pulls A's feed over HTTP and applies it ---
    let replicator = Replicator::new(
        store_b.clone(),
        registry.clone() as Arc<dyn OwnershipRegistry>,
    );
    let peer = Peer {
        node: OWNER_NODE as i64,
        client: FeedClient::new(&base, Some(TOKEN.to_string())),
    };
    let applied = adapter_federation::poll_peer(&replicator, &peer, 500)
        .await
        .unwrap();
    assert!(
        applied >= 3,
        "user + demos + post replicated, got {applied}"
    );

    // B now holds A's post, fetched purely over the network.
    assert_eq!(
        PostStore::get(&*store_b, post.id)
            .await
            .unwrap()
            .unwrap()
            .title,
        "Hello over HTTP"
    );

    // --- a second poll resumes from the cursor and applies nothing new ---
    let again = adapter_federation::poll_peer(&replicator, &peer, 500)
        .await
        .unwrap();
    assert_eq!(again, 0, "cursor resume: nothing new on the second poll");
}
