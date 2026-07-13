//! Adversarial coverage of delegated account minting + login: a non-issuer node
//! forwards sign-up and sign-in to a trusted issuer. These tests try to *break* the
//! trust boundary — forge the requesting node, flood the rate limits, grind
//! passwords, and mint into the wrong namespace. Gated on `TEST_DATABASE_URL`.

use std::sync::Arc;

use adapter_federation::command::auth_rate_limiter::AuthRateLimiter;
use adapter_federation::command::mint_rate_limiter::MintRateLimiter;
use adapter_federation::command::replay_guard::ReplayGuard;
use adapter_federation::command::signed_command::SignedCommand;
use adapter_federation::{command_router, Command, CommandState, InMemoryRateLimitStore};
use adapter_media_local::LocalMediaStore;
use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_postgres::PostgresStore;
use app::{Clock, Services};
use domain::{origin_node, NodeId, Timestamp};
use federation::{InMemoryRegistry, NodeKeypair, OwnershipRegistry};

const ISSUER: u16 = 7;
const REQUESTER: u16 = 8;
const GHOST: u16 = 9; // a node whose key the fleet never published

struct FixedClock(i64);
impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        Timestamp(self.0)
    }
}

fn mk_services(store: Arc<PostgresStore>) -> Services {
    let dir = std::env::temp_dir().join(format!("demfed-del-{}", std::process::id()));
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

/// POST a signed command and return the raw HTTP status + body text, so a test can
/// assert on 401/422/429 directly (a real forwarder collapses these).
async fn post_signed(base: &str, token: &str, signed: &SignedCommand) -> (u16, String) {
    let resp = reqwest::Client::new()
        .post(format!("{base}/federation/command"))
        .bearer_auth(token)
        .json(signed)
        .send()
        .await
        .unwrap();
    let status = resp.status().as_u16();
    (status, resp.text().await.unwrap_or_default())
}

/// Bring up an issuer command server with deliberately tiny rate caps so the
/// adversarial floods hit them quickly. Returns (base_url, requester keypair, token).
async fn spawn_issuer(store: Arc<PostgresStore>) -> (String, Arc<NodeKeypair>, String) {
    let services = mk_services(store);
    let registry = Arc::new(InMemoryRegistry::new());

    // Only the REQUESTER's key is published — GHOST's is not, so its commands can't
    // authenticate.
    let requester = Arc::new(NodeKeypair::generate(NodeId(REQUESTER)));
    registry
        .publish_key(NodeId(REQUESTER), &requester.public().to_hex())
        .await
        .unwrap();

    let token = "cluster-secret".to_string();
    let state = CommandState {
        node: NodeId(ISSUER),
        services,
        token: Some(token.clone()),
        registry: registry.clone() as Arc<dyn OwnershipRegistry>,
        replay_guard: Arc::new(ReplayGuard::in_memory()),
        // Tiny caps: 2 mints/window, 2 login attempts/window. In-memory store is fine
        // for a single-process test; production uses the durable Postgres-backed one.
        mint_rate_limiter: Arc::new(MintRateLimiter::new(
            Arc::new(InMemoryRateLimitStore::new()),
            2,
            3_600,
        )),
        auth_rate_limiter: Arc::new(AuthRateLimiter::new(
            Arc::new(InMemoryRateLimitStore::new()),
            2,   // per-handle: brute-force cap
            100, // per-node: spraying cap (high, so the per-handle assertions fire)
            300,
        )),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, command_router(state)).await.unwrap();
    });
    (format!("http://{addr}"), requester, token)
}

#[tokio::test]
async fn delegated_minting_and_login_resist_abuse() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    let store = Arc::new(PostgresStore::connect(&url, NodeId(ISSUER)).await.unwrap());
    sqlx::query("TRUNCATE users, id_counters, outbox, replication_cursor, command_nonces")
        .execute(store.pool())
        .await
        .unwrap();

    let (base, requester, token) = spawn_issuer(store.clone()).await;
    let ghost = NodeKeypair::generate(NodeId(GHOST));

    let mint = |handle: &str| Command::MintAccount {
        handle: handle.to_string(),
        email: format!("{handle}@example.com"),
        password: "correct horse battery".to_string(),
    };

    // 1) A command from a node whose key the fleet never published is rejected as
    //    unauthenticated — a bare token-holder can't mint.
    let (status, _) = post_signed(&base, &token, &SignedCommand::sign(&ghost, &mint("mallory"))).await;
    assert_eq!(status, 401, "ghost-node command must be unauthorized");

    // 2) A legitimately-signed mint succeeds, and the account is minted in the
    //    ISSUER's id namespace — the requester cannot influence the home node.
    let (status, body) =
        post_signed(&base, &token, &SignedCommand::sign(&requester, &mint("alice"))).await;
    assert_eq!(status, 200, "authenticated mint should succeed: {body}");
    let outcome: adapter_federation::CommandOutcome = serde_json::from_str(&body).unwrap();
    let adapter_federation::CommandOutcome::AccountMinted { id } = outcome else {
        panic!("expected AccountMinted, got {outcome:?}");
    };
    assert_eq!(
        origin_node(id),
        NodeId(ISSUER),
        "the account must be homed on the issuer, not the requester"
    );

    // 3) Mint rate limit: the cap is 2/window; a burst is throttled with 429, not
    //    silently accepted. (One already spent above.)
    let (s2, _) = post_signed(&base, &token, &SignedCommand::sign(&requester, &mint("bob"))).await;
    assert_eq!(s2, 200, "second mint within cap");
    let (s3, _) = post_signed(&base, &token, &SignedCommand::sign(&requester, &mint("carol"))).await;
    assert_eq!(s3, 429, "third mint over the cap must be throttled");

    // 4) Delegated login: correct password authenticates and returns the account id.
    let good_login = Command::Authenticate {
        handle: "alice".to_string(),
        password: "correct horse battery".to_string(),
    };
    let (status, body) = post_signed(&base, &token, &SignedCommand::sign(&requester, &good_login)).await;
    assert_eq!(status, 200, "correct delegated login should succeed: {body}");
    let outcome: adapter_federation::CommandOutcome = serde_json::from_str(&body).unwrap();
    assert_eq!(
        outcome,
        adapter_federation::CommandOutcome::Authenticated { id },
        "the home issuer verifies and returns the same account id it minted"
    );

    // 5) Brute force: wrong-password guesses are refused, and the per-handle cap (2)
    //    throttles the grind with 429 rather than allowing unlimited attempts.
    let guess = |pw: &str| Command::Authenticate {
        handle: "alice".to_string(),
        password: pw.to_string(),
    };
    // The good login above counts as attempt #1; one more wrong guess is the 2nd…
    let (g2, _) = post_signed(&base, &token, &SignedCommand::sign(&requester, &guess("wrong-1"))).await;
    assert_eq!(g2, 422, "a wrong password is a merits rejection, opaque");
    // …and the 3rd attempt for this handle is throttled.
    let (g3, _) = post_signed(&base, &token, &SignedCommand::sign(&requester, &guess("wrong-2"))).await;
    assert_eq!(g3, 429, "delegated login is brute-force capped per account");

    // 6) An unknown handle is refused with the SAME opaque rejection as a wrong
    //    password (no account-existence leak) — a fresh handle dodges the per-handle cap.
    let unknown = Command::Authenticate {
        handle: "nobody".to_string(),
        password: "whatever".to_string(),
    };
    let (s, _) = post_signed(&base, &token, &SignedCommand::sign(&requester, &unknown)).await;
    assert_eq!(s, 422, "unknown handle looks identical to a wrong password");
}
