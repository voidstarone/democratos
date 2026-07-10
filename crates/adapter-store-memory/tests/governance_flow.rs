//! End-to-end use-case tests through the `Services` facade, wired to the
//! in-memory adapter. These exercise the full stack (app + ports + adapter) and
//! double as executable documentation of the flood-resistance story.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::{Clock, EnfranchiseOutcome, Services};
use domain::{FranchiseCriteria, ProposalKind, ProposalStatus, Tier, Timestamp};

const DAY: i64 = Timestamp::SECONDS_PER_DAY;

fn build() -> (Services, Arc<MemoryStore>, Arc<FixedClock>) {
    let store = Arc::new(MemoryStore::new());
    let clock = Arc::new(FixedClock::at(Timestamp(1_000 * DAY)));
    let services = Services {
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
        media: store.clone(),
        recommender: Arc::new(MemoryRecommender::default()),
        nsfw_scanner: Arc::new(HeuristicNsfwScanner),
        age_verifier: Arc::new(AutoApproveAgeVerifier),
        requires_age_verification: false,
        require_signatures: false,
        clock: clock.clone(),
    };
    (services, store, clock)
}

/// A flood of brand-new accounts cannot vote: they fail Layer 1 on every axis.
#[tokio::test]
async fn fresh_flood_cannot_enfranchise() {
    let (svc, _store, _clock) = build();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    for i in 0..50 {
        let u = svc.register_user(&format!("flood{i}")).await.unwrap();
        svc.join(u.id, demos.id).await.unwrap();
        let outcome = svc.request_enfranchisement(u.id, demos.id).await.unwrap();
        assert!(matches!(outcome, EnfranchiseOutcome::NotEligible(_)));
    }

    // Only the founder is a voter; the demos stays in Seed.
    assert_eq!(svc.memberships.voter_count(demos.id).await.unwrap(), 1);
}

/// Even once a flood becomes *eligible*, the Layer-2 rate cap throttles how many
/// can be admitted in a single 30-day window. The electorate cannot be flipped.
#[tokio::test]
async fn rate_cap_throttles_a_qualified_flood() {
    let (svc, _store, clock) = build();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    // 50 members who all join today and earn contribution.
    let mut flood = Vec::new();
    for i in 0..50 {
        let u = svc.register_user(&format!("m{i}")).await.unwrap();
        svc.join(u.id, demos.id).await.unwrap();
        svc.record_contribution(u.id, demos.id, 10).await.unwrap();
        flood.push(u.id);
    }

    // Age past the account + membership requirements.
    clock.advance_days(40);

    let mut admitted = 0;
    let mut queued = 0;
    for id in &flood {
        match svc.request_enfranchisement(*id, demos.id).await.unwrap() {
            EnfranchiseOutcome::Admitted => admitted += 1,
            EnfranchiseOutcome::Queued => queued += 1,
            EnfranchiseOutcome::NotEligible(e) => panic!("should be eligible: {:?}", e.unmet),
        }
    }

    // Founder = 1 voter. Cap = max(5, ceil(10% of 1)) = 5 this window.
    assert_eq!(admitted, 5, "rate cap should admit exactly the floor of 5");
    assert_eq!(queued, 45, "the rest are eligible but queued");
    assert_eq!(svc.memberships.voter_count(demos.id).await.unwrap(), 6);
}

/// A genuine, gradual shift IS allowed: across successive windows the queue
/// drains. Democracy is preserved — takeover is slowed, not blocked.
#[tokio::test]
async fn queue_drains_across_windows() {
    let (svc, _store, clock) = build();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    let mut flood = Vec::new();
    for i in 0..8 {
        let u = svc.register_user(&format!("m{i}")).await.unwrap();
        svc.join(u.id, demos.id).await.unwrap();
        svc.record_contribution(u.id, demos.id, 10).await.unwrap();
        flood.push(u.id);
    }
    clock.advance_days(40);

    // request_enfranchisement is idempotent (already-voters return Admitted), so
    // we measure the real effect: how the voter roll grows window over window.
    async fn admit_round(svc: &Services, ids: &[domain::UserId], demos: domain::DemosId) {
        for id in ids {
            svc.request_enfranchisement(*id, demos).await.unwrap();
        }
    }

    admit_round(&svc, &flood, demos.id).await;
    // Founder (1) + floor of 5 admitted = 6.
    assert_eq!(svc.memberships.voter_count(demos.id).await.unwrap(), 6);

    // New window; the previously-queued members can now be admitted.
    clock.advance_days(31);
    admit_round(&svc, &flood, demos.id).await;
    // Remaining 3 drain in -> 9 voters total. The shift happened, just slowly.
    assert_eq!(svc.memberships.voter_count(demos.id).await.unwrap(), 9);
}

/// Seed-phase demos cannot amend their constitution (training wheels).
#[tokio::test]
async fn constitutional_change_forbidden_in_seed() {
    let (svc, _store, _clock) = build();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    let err = svc
        .open_proposal(
            founder.id,
            demos.id,
            ProposalKind::AmendCriteria {
                proposed: FranchiseCriteria::platform_default(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, app::OpenProposalError::ConstitutionalForbiddenInSeed));
}

/// A full moderation proposal: open, vote, close, pass.
#[tokio::test]
async fn moderation_proposal_passes_by_majority() {
    let (svc, _store, clock) = build();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    // Build an electorate of 5 voters (founder + 4 admitted).
    let mut voters = vec![founder.id];
    for i in 0..4 {
        let u = svc.register_user(&format!("v{i}")).await.unwrap();
        svc.join(u.id, demos.id).await.unwrap();
        svc.record_contribution(u.id, demos.id, 10).await.unwrap();
        voters.push(u.id);
    }
    clock.advance_days(40);
    for id in &voters[1..] {
        assert_eq!(
            svc.request_enfranchisement(*id, demos.id).await.unwrap(),
            EnfranchiseOutcome::Admitted
        );
    }
    assert_eq!(svc.memberships.voter_count(demos.id).await.unwrap(), 5);

    let proposal = svc
        .open_proposal(
            founder.id,
            demos.id,
            ProposalKind::RemoveContent {
                target: "post:42".into(),
            },
        )
        .await
        .unwrap();

    // 3 aye, 1 nay — clear majority, ample turnout.
    svc.cast_vote(proposal.id, voters[0], true, None)
        .await
        .unwrap();
    svc.cast_vote(proposal.id, voters[1], true, None)
        .await
        .unwrap();
    svc.cast_vote(proposal.id, voters[2], true, None)
        .await
        .unwrap();
    svc.cast_vote(proposal.id, voters[3], false, None)
        .await
        .unwrap();

    // A voter cannot vote twice.
    assert!(matches!(
        svc.cast_vote(proposal.id, voters[0], false, None).await,
        Err(app::CastVoteError::Store(app::StoreError::AlreadyVoted))
    ));

    // A proposal cannot be closed until its voting window has elapsed (moderation
    // votes run 3 days) — closing early would let a voter freeze the tally.
    assert!(matches!(
        svc.close_proposal(proposal.id).await,
        Err(app::CloseProposalError::VotingWindowOpen)
    ));
    clock.advance_days(3);

    let status = svc.close_proposal(proposal.id).await.unwrap();
    match status {
        ProposalStatus::Passed { effective_at } => {
            // Non-constitutional: effective at the close moment, no timelock.
            assert_eq!(effective_at, clock.now());
        }
        other => panic!("expected Passed, got {other:?}"),
    }
}

/// Two open proposals to remove the same post must never coexist: the second
/// identical proposal is rejected, but a different target is fine, and once the
/// first is closed the same intent may be proposed again.
#[tokio::test]
async fn duplicate_open_proposal_is_rejected() {
    let (svc, _store, clock) = build();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    let remove_42 = || ProposalKind::RemoveContent {
        target: "post:42".into(),
    };

    let first = svc
        .open_proposal(founder.id, demos.id, remove_42())
        .await
        .unwrap();

    // A second proposal with the same intent is refused while the first is open.
    let err = svc
        .open_proposal(founder.id, demos.id, remove_42())
        .await
        .unwrap_err();
    assert!(matches!(err, app::OpenProposalError::DuplicateOpenProposal));

    // A proposal targeting a different post is unaffected.
    svc.open_proposal(
        founder.id,
        demos.id,
        ProposalKind::RemoveContent {
            target: "post:99".into(),
        },
    )
    .await
    .unwrap();

    // Once the first closes, the intent is no longer "open" and may recur. Closing
    // requires the voting window (3 days for moderation) to have elapsed first.
    clock.advance_days(3);
    svc.close_proposal(first.id).await.unwrap();
    svc.open_proposal(founder.id, demos.id, remove_42())
        .await
        .unwrap();
}

/// A non-voter cannot open proposals or vote.
#[tokio::test]
async fn non_voter_is_locked_out_of_governance() {
    let (svc, _store, _clock) = build();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();

    let lurker = svc.register_user("lurker").await.unwrap();
    svc.join(lurker.id, demos.id).await.unwrap();
    assert_eq!(
        svc.memberships
            .get(lurker.id, demos.id)
            .await
            .unwrap()
            .unwrap()
            .tier,
        Tier::Member
    );

    let err = svc
        .open_proposal(
            lurker.id,
            demos.id,
            ProposalKind::RemoveContent {
                target: "post:1".into(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, app::OpenProposalError::NotAVoter));
}
