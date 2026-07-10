//! Backfill import: IDs are preserved, relationships stay intact, the import is
//! idempotent, and the ID counters advance so later `create`s don't collide with
//! imported IDs. Gated on `TEST_DATABASE_URL`.

use std::sync::Arc;

use adapter_store_postgres::{ImportCounts, ImportData, PostgresStore, VoteRow};
use app::{DemosStore, MembershipStore, ProposalStore, UserStore, VoteStore};
use domain::{
    compose_id, Demos, Membership, NodeId, Proposal, ProposalKind, ProposalStatus, Tier, Timestamp,
    User,
};

// Import as the single-box node 0, so imported IDs (minted by node 0) share the
// key space this node mints in — the case where counter advancement matters.
const NODE: u16 = 0;

fn user(id: u64, handle: &str) -> User {
    User {
        id: domain::UserId(id),
        handle: handle.to_string(),
        email: None,
        password_hash: None,
        created_at: Timestamp(1),
        is_age_verified: false,
        public_key: None,
        feed_paging: domain::FeedPaging::Auto,
        is_franchise_barred: false,
    }
}

fn demos(id: u64, slug: &str, founder: u64) -> Demos {
    Demos::new(
        domain::DemosId(id),
        slug,
        slug.to_uppercase(),
        domain::UserId(founder),
        Timestamp(1),
    )
}

fn proposal(id: u64, demos: u64, proposer: u64) -> Proposal {
    Proposal {
        id: domain::ProposalId(id),
        demos_id: domain::DemosId(demos),
        proposer: domain::UserId(proposer),
        kind: ProposalKind::AddRule {
            text: "be kind".into(),
        },
        opened_at: Timestamp(1),
        closes_at: Timestamp(1_000),
        status: ProposalStatus::Open,
        applied: false,
        rev: 0,
    }
}

fn membership(user: u64, demos: u64) -> Membership {
    let mut m = Membership::joined(domain::UserId(user), domain::DemosId(demos), Timestamp(1));
    m.tier = Tier::Voter;
    m.enfranchised_at = Some(Timestamp(1));
    m
}

#[tokio::test]
async fn import_preserves_ids_relationships_and_advances_counters() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("skipping: TEST_DATABASE_URL not set");
        return;
    };
    let store = Arc::new(PostgresStore::connect(&url, NodeId(NODE)).await.unwrap());
    sqlx::query(
        "TRUNCATE users, demoi, memberships, proposals, votes, post_votes, rules, posts, \
         comments, reports, trials, jury_ballots, id_counters, outbox, replication_cursor",
    )
    .execute(store.pool())
    .await
    .unwrap();

    // IDs 10 and 20 leave a gap: the next create must jump past them, not reuse.
    let data = ImportData {
        users: vec![user(10, "founder"), user(11, "voter")],
        demoi: vec![demos(20, "rust", 10)],
        memberships: vec![membership(11, 20)],
        proposals: vec![proposal(30, 20, 10)],
        votes: vec![VoteRow {
            proposal: 30,
            voter: 11,
            aye: true,
            weight: 1,
        }],
        ..Default::default()
    };

    let counts = store.import(&data).await.unwrap();
    assert_eq!(
        counts,
        ImportCounts {
            users: 2,
            demoi: 1,
            memberships: 1,
            proposals: 1,
            votes: 1,
            ..Default::default()
        }
    );

    // IDs preserved exactly.
    let founder = UserStore::get(&*store, domain::UserId(10))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(founder.handle, "founder");
    let d = DemosStore::get(&*store, domain::DemosId(20))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(d.slug, "rust");
    assert_eq!(d.founder_id.0, 10);
    // Relationship intact: the vote points at the imported proposal + voter.
    assert!(
        VoteStore::has_voted(&*store, domain::ProposalId(30), domain::UserId(11))
            .await
            .unwrap()
    );
    // Membership tier survived the lifted-column mapping.
    let m = MembershipStore::get(&*store, domain::UserId(11), domain::DemosId(20))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(m.tier, Tier::Voter);

    // Idempotent: a second import inserts nothing.
    let again = store.import(&data).await.unwrap();
    assert_eq!(again.total(), 0, "re-import must be a no-op");

    // Counters advanced: a freshly created user gets an ID past the imported 11,
    // and a new proposal past 30 — no collision with imported IDs.
    let fresh_user = UserStore::create(&*store, "new", None, None, Timestamp(2))
        .await
        .unwrap();
    assert!(
        fresh_user.id.0 > 11,
        "new user id {} must clear imported ids",
        fresh_user.id.0
    );
    assert_eq!(fresh_user.id.0, compose_id(NodeId(NODE), 12));
    let fresh_prop = ProposalStore::create(
        &*store,
        domain::DemosId(20),
        domain::UserId(10),
        ProposalKind::AddRule { text: "x".into() },
        Timestamp(2),
        Timestamp(9_999),
    )
    .await
    .unwrap();
    assert!(
        fresh_prop.id.0 > 30,
        "new proposal id must clear imported ids"
    );
}
