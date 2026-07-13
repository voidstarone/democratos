//! Write forwarding: a vote use-case routed to the community's owner. Covers
//! local execution (we own it), HTTP forwarding to the owner, the owner rejecting
//! a double vote, and fail-closed behaviour when there is no reachable owner.
//! Gated on `TEST_DATABASE_URL`.

use std::sync::Arc;
use std::time::Duration;

use adapter_federation::{
    command_router, Command, CommandClient, CommandOutcome, CommandState, ForwardError,
    HttpCommandTransport, WriteRouter,
};
use adapter_media_local::LocalMediaStore;
use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_postgres::PostgresStore;
use app::{Clock, DemosStore, MembershipStore, ProposalStore, UserStore, VoteStore};
use app::Services;
use domain::{Membership, NodeId, ProposalKind, Tier, Timestamp, UserId};
use federation::{InMemoryRegistry, NodeKeypair, OwnershipRegistry};

const OWNER: u16 = 7;
const OTHER: u16 = 8;

struct FixedClock(i64);
impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp(self.0)
    }
}

fn mk_services(store: Arc<PostgresStore>) -> Services {
    let dir = std::env::temp_dir().join(format!("demfed-media-{}", std::process::id()));
    let media = Arc::new(LocalMediaStore::new(dir).unwrap());
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
        media,
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

/// Insert a voter membership directly (bypassing the time-gated enfranchisement
/// flow) so the vote use-case has an eligible voter.
async fn add_voter(store: &PostgresStore, user: UserId, demos: domain::DemosId) {
    let mut m = Membership::joined(user, demos, Timestamp(1));
    m.tier = Tier::Voter;
    m.enfranchised_at = Some(Timestamp(1));
    store.upsert(m).await.unwrap();
}

#[tokio::test]
async fn votes_route_to_the_owner_and_fail_closed_without_one() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    let store = Arc::new(PostgresStore::connect(&url, NodeId(OWNER)).await.unwrap());
    sqlx::query(
        "TRUNCATE users, demoi, memberships, proposals, votes, id_counters, outbox, replication_cursor",
    )
    .execute(store.pool())
    .await
    .unwrap();
    let services = mk_services(store.clone());

    // Owner community + an open proposal + two voters.
    let founder = UserStore::create(&*store, "founder", None, None, Timestamp(1))
        .await
        .unwrap();
    let demos = DemosStore::create(&*store, "rust", "R", founder.id, Timestamp(1))
        .await
        .unwrap();
    let v1 = UserStore::create(&*store, "v1", None, None, Timestamp(1))
        .await
        .unwrap();
    let v2 = UserStore::create(&*store, "v2", None, None, Timestamp(1))
        .await
        .unwrap();
    add_voter(&store, v1.id, demos.id).await;
    add_voter(&store, v2.id, demos.id).await;
    let proposal = ProposalStore::create(
        &*store,
        demos.id,
        founder.id,
        ProposalKind::AddRule {
            text: "be kind".into(),
        },
        Timestamp(1),
        Timestamp(999_999),
    )
    .await
    .unwrap();

    // Control plane: OWNER node owns the community. Both nodes publish their keys
    // so the owner can authenticate a command the forwarding node (OTHER) signs.
    let registry = Arc::new(InMemoryRegistry::new());
    let kp = NodeKeypair::generate(NodeId(OWNER));
    registry
        .publish_key(NodeId(OWNER), &kp.public().to_hex())
        .await
        .unwrap();
    let kp_other = Arc::new(NodeKeypair::generate(NodeId(OTHER)));
    registry
        .publish_key(NodeId(OTHER), &kp_other.public().to_hex())
        .await
        .unwrap();
    registry.claim(demos.id.0, NodeId(OWNER)).await.unwrap();

    // Owner runs a command server.
    let token = Some("t".to_string());
    let state = CommandState {
        node: NodeId(OWNER),
        services: services.clone(),
        token: token.clone(),
        registry: registry.clone() as Arc<dyn OwnershipRegistry>,
        replay_guard: Arc::new(adapter_federation::command::replay_guard::ReplayGuard::in_memory()),
        mint_rate_limiter: Arc::new(
            adapter_federation::command::mint_rate_limiter::MintRateLimiter::new(
                Arc::new(adapter_federation::InMemoryRateLimitStore::new()),
                30,
                3_600,
            ),
        ),
        auth_rate_limiter: Arc::new(
            adapter_federation::command::auth_rate_limiter::AuthRateLimiter::new(
                Arc::new(adapter_federation::InMemoryRateLimitStore::new()),
                10,
                100,
                300,
            ),
        ),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, command_router(state)).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    let base = format!("http://{addr}");

    // --- forwarding node (OTHER) routes a vote to the OWNER over HTTP ---
    let transport = Arc::new(HttpCommandTransport::new().with_peer(
        NodeId(OWNER),
        CommandClient::new(&base, token.clone(), kp_other.clone()),
    ));
    let router_other = WriteRouter::new(
        NodeId(OTHER),
        services.clone(),
        registry.clone() as Arc<dyn OwnershipRegistry>,
        transport,
    );
    let outcome = router_other
        .submit(Command::CastVote {
            proposal: proposal.id.0,
            voter: v1.id.0,
            aye: true,
            sig: None,
        })
        .await
        .expect("forwarded vote succeeds");
    assert_eq!(outcome, CommandOutcome::Voted);
    assert!(
        VoteStore::has_voted(&*store, proposal.id, v1.id)
            .await
            .unwrap(),
        "the owner recorded the forwarded vote"
    );

    // --- forwarding the SAME vote again → owner rejects (AlreadyVoted) ---
    let dup = router_other
        .submit(Command::CastVote {
            proposal: proposal.id.0,
            voter: v1.id.0,
            aye: true,
            sig: None,
        })
        .await;
    assert!(matches!(dup, Err(ForwardError::Rejected(_))), "got {dup:?}");

    // --- the owner node executes locally (no forward) ---
    let router_owner = WriteRouter::new(
        NodeId(OWNER),
        services.clone(),
        registry.clone() as Arc<dyn OwnershipRegistry>,
        Arc::new(HttpCommandTransport::new()), // unused: it owns the community
    );
    router_owner
        .submit(Command::CastVote {
            proposal: proposal.id.0,
            voter: v2.id.0,
            aye: false,
            sig: None,
        })
        .await
        .expect("local vote succeeds");
    assert!(VoteStore::has_voted(&*store, proposal.id, v2.id)
        .await
        .unwrap());

    // --- fail-closed: a community with no owner refuses writes ---
    let orphan = DemosStore::create(&*store, "orphan", "O", founder.id, Timestamp(1))
        .await
        .unwrap();
    add_voter(&store, v1.id, orphan.id).await;
    let orphan_prop = ProposalStore::create(
        &*store,
        orphan.id,
        founder.id,
        ProposalKind::AddRule { text: "x".into() },
        Timestamp(1),
        Timestamp(999_999),
    )
    .await
    .unwrap();
    // orphan is never claimed → unowned.
    let refused = router_other
        .submit(Command::CastVote {
            proposal: orphan_prop.id.0,
            voter: v1.id.0,
            aye: true,
            sig: None,
        })
        .await;
    assert!(
        matches!(refused, Err(ForwardError::Unowned)),
        "got {refused:?}"
    );
    assert!(
        !VoteStore::has_voted(&*store, orphan_prop.id, v1.id)
            .await
            .unwrap(),
        "a write to an unowned community must not be applied"
    );

    // --- fail-closed: owner known but unreachable (no route) ---
    registry.claim(orphan.id.0, NodeId(99)).await.unwrap(); // node 99 has no route
    let unreachable = router_other
        .submit(Command::CastVote {
            proposal: orphan_prop.id.0,
            voter: v1.id.0,
            aye: true,
            sig: None,
        })
        .await;
    assert!(
        matches!(unreachable, Err(ForwardError::OwnerUnreachable(_))),
        "got {unreachable:?}"
    );
}
