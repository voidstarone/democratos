//! The delivery-facing write gateway: the web/CLI adapters submit votes through
//! the `app::GovernanceWrites` port, which the federation implements as
//! `FederatedWrites` (route to owner; locally-owned → quorum-of-2 sync). This
//! proves a vote entered via the port is recorded on the owner AND its standby,
//! and that a store outcome keeps its TYPED `StoreError` through the port (so
//! the UI still says "already voted", not a stringified message). Two Postgres
//! DBs stand in for owner + standby. Gated on `TEST_DATABASE_URL`.

use std::sync::Arc;
use std::time::Duration;

use adapter_federation::{
    ingest_router, FederatedWrites, HttpCommandTransport, IngestClient, IngestState, Replicator,
    SyncVoteExecutor, WriteRouter,
};
use adapter_media_local::LocalMediaStore;
use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_postgres::PostgresStore;
use app::{CastVoteError, StoreError};
use app::{Clock, DemosStore, MembershipStore, ProposalStore, UserStore, VoteStore};
use app::{GovernanceWrites, Services};
use domain::{Membership, NodeId, ProposalKind, Tier, Timestamp, UserId};
use federation::{InMemoryRegistry, NodeKeypair, OwnershipRegistry};
use sqlx::Connection;

const OWNER: u16 = 7;
const STANDBY: u16 = 8;

struct FixedClock(i64);
impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp(self.0)
    }
}

fn mk_services(store: Arc<PostgresStore>) -> Services {
    let dir = std::env::temp_dir().join(format!("demgw-media-{}", std::process::id()));
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
        post_votes: store.clone(),
        comment_votes: store.clone(),
        media: Arc::new(LocalMediaStore::new(dir).unwrap()),
        recommender: Arc::new(MemoryRecommender::default()),
        nsfw_scanner: Arc::new(HeuristicNsfwScanner),
        age_verifier: Arc::new(AutoApproveAgeVerifier),
        requires_age_verification: false,
        require_signatures: false,
        notifier: Arc::new(adapter_notify::LogNotifier::new()),
        public_base_url: "http://localhost".to_string(),
        invite_token_ttl_days: 7,
        clock: Arc::new(FixedClock(1000)),
    }
}

fn sibling_url(url: &str, db: &str) -> String {
    let (prefix, _) = url.rsplit_once('/').unwrap();
    format!("{prefix}/{db}")
}

async fn add_voter(store: &PostgresStore, user: UserId, demos: domain::DemosId) {
    let mut m = Membership::joined(user, demos, Timestamp(1));
    m.tier = Tier::Voter;
    m.enfranchised_at = Some(Timestamp(1));
    store.upsert(m).await.unwrap();
}

#[tokio::test]
async fn a_vote_via_the_port_is_owned_synced_and_keeps_typed_errors() {
    let Ok(url_a) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    let db_b = "democratos_gateway_standby";
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

    let store_a = Arc::new(PostgresStore::connect(&url_a, NodeId(OWNER)).await.unwrap());
    let store_b = Arc::new(
        PostgresStore::connect(&url_b, NodeId(STANDBY))
            .await
            .unwrap(),
    );
    sqlx::query(
        "TRUNCATE users, demoi, memberships, proposals, votes, id_counters, outbox, replication_cursor",
    )
    .execute(store_a.pool())
    .await
    .unwrap();
    let services = mk_services(store_a.clone());

    // Owner data: a community, a proposal, one eligible voter.
    let founder = UserStore::create(&*store_a, "f", None, None, Timestamp(1))
        .await
        .unwrap();
    let demos = DemosStore::create(&*store_a, "rust", "R", founder.id, Timestamp(1))
        .await
        .unwrap();
    let v1 = UserStore::create(&*store_a, "v1", None, None, Timestamp(1))
        .await
        .unwrap();
    add_voter(&store_a, v1.id, demos.id).await;
    let proposal = ProposalStore::create(
        &*store_a,
        demos.id,
        founder.id,
        ProposalKind::AddRule { text: "k".into() },
        Timestamp(1),
        Timestamp(999_999),
    )
    .await
    .unwrap();

    // Control plane: OWNER owns d/rust, STANDBY is its standby.
    let owner_kp = Arc::new(NodeKeypair::generate(NodeId(OWNER)));
    let registry = Arc::new(InMemoryRegistry::new());
    registry
        .publish_key(NodeId(OWNER), &owner_kp.public().to_hex())
        .await
        .unwrap();
    registry.claim(demos.id.0, NodeId(OWNER)).await.unwrap();
    registry
        .set_standby(demos.id.0, NodeId(STANDBY))
        .await
        .unwrap();

    // In a real cluster the standby is kept current by the ordered puller, so by
    // the time anyone votes it already holds the community and the proposal. Prime
    // it here with the owner's state; otherwise the forwarded vote's parent
    // proposal is absent and `authorize` rejects the vote as `Unowned` (the sync
    // push then fails closed) — a precondition the puller normally satisfies.
    let reg_dyn: Arc<dyn OwnershipRegistry> = registry.clone();
    let seed =
        adapter_federation::changes_since(&*store_a, &owner_kp, reg_dyn.as_ref(), 0, 10_000)
            .await
            .unwrap();
    Replicator::new(store_b.clone(), reg_dyn.clone())
        .apply_pushed(&seed)
        .await
        .unwrap();

    // Standby runs the ingest server.
    let token = Some("t".to_string());
    let replicator = Arc::new(Replicator::new(
        store_b.clone(),
        registry.clone() as Arc<dyn OwnershipRegistry>,
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            ingest_router(IngestState {
                replicator,
                token: Some("t".into()),
            }),
        )
        .await
        .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Build the gateway exactly as the composition root does: a WriteRouter that
    // executes locally-owned votes through the SyncVoteExecutor.
    let sync = SyncVoteExecutor::new(
        NodeId(OWNER),
        services.clone(),
        store_a.clone(),
        owner_kp.clone(),
        registry.clone() as Arc<dyn OwnershipRegistry>,
    )
    .with_standby(
        NodeId(STANDBY),
        IngestClient::new(format!("http://{addr}"), token.clone()),
    );
    let router = WriteRouter::new(
        NodeId(OWNER),
        services.clone(),
        registry.clone() as Arc<dyn OwnershipRegistry>,
        Arc::new(HttpCommandTransport::new()),
    )
    .with_sync(Arc::new(sync));
    let writes: Arc<dyn GovernanceWrites> = Arc::new(FederatedWrites::new(router));

    // --- a vote through the port is recorded on owner AND standby ---
    writes
        .cast_vote(proposal.id, v1.id, true, None)
        .await
        .expect("vote accepted");
    assert!(VoteStore::has_voted(&*store_a, proposal.id, v1.id)
        .await
        .unwrap());
    assert!(
        VoteStore::has_voted(&*store_b, proposal.id, v1.id)
            .await
            .unwrap(),
        "the standby has the vote synchronously (quorum of 2)"
    );

    // --- a double vote keeps its TYPED error through the port ---
    let err = writes
        .cast_vote(proposal.id, v1.id, true, None)
        .await
        .unwrap_err();
    assert!(
        matches!(err, CastVoteError::Store(StoreError::AlreadyVoted)),
        "expected the typed AlreadyVoted to survive the gateway, got {err:?}"
    );
}
