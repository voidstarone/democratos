//! Per-user signed governance actions (the open-federation trust root): once an
//! account enrols an Ed25519 signing key, its votes must carry a signature that
//! verifies against that key — so no node can forge them — while accounts without
//! a key keep working during rollout.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::{identity, Services};
use domain::{ProposalKind, Timestamp};
use ed25519_dalek::{Signer, SigningKey};

const DAY: i64 = Timestamp::SECONDS_PER_DAY;

fn build() -> Services {
    let store = Arc::new(MemoryStore::new());
    let clock = Arc::new(FixedClock::at(Timestamp(1_000 * DAY)));
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
        media: store.clone(),
        recommender: Arc::new(MemoryRecommender::default()),
        nsfw_scanner: Arc::new(HeuristicNsfwScanner),
        age_verifier: Arc::new(AutoApproveAgeVerifier),
        requires_age_verification: false,
        require_signatures: false,
        clock,
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    s
}

/// A device-held keypair: returns (signing key, hex public key).
fn user_key(seed: u8) -> (SigningKey, String) {
    let sk = SigningKey::from_bytes(&[seed; 32]);
    let pub_hex = hex(sk.verifying_key().as_bytes());
    (sk, pub_hex)
}

#[tokio::test]
async fn an_enrolled_account_must_sign_its_votes() {
    let svc = build();

    // A founder (voter of their own demos) opens a proposal.
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let proposal = svc
        .open_proposal(
            founder.id,
            demos.id,
            ProposalKind::AddRule {
                text: "be kind".into(),
            },
        )
        .await
        .unwrap();

    // Enrol the founder's signing key.
    let (sk, pub_hex) = user_key(1);
    svc.enroll_public_key(founder.id, &pub_hex).await.unwrap();

    let msg = identity::vote_message::vote_message(proposal.id.0, true);
    let good_sig = hex(&sk.sign(msg.as_bytes()).to_bytes());

    // No signature → rejected now that the account has a key.
    assert!(
        svc.cast_vote(proposal.id, founder.id, true, None)
            .await
            .is_err(),
        "an enrolled account cannot vote unsigned"
    );

    // A signature from a *different* key (the forgery a rogue node would attempt)
    // → rejected.
    let (attacker_sk, _) = user_key(2);
    let forged = hex(&attacker_sk.sign(msg.as_bytes()).to_bytes());
    assert!(
        svc.cast_vote(proposal.id, founder.id, true, Some(&forged))
            .await
            .is_err(),
        "a signature by another key must not authorize the vote"
    );

    // A signature over a *different* decision (nay) can't authorize an aye.
    let nay_msg = identity::vote_message::vote_message(proposal.id.0, false);
    let nay_sig = hex(&sk.sign(nay_msg.as_bytes()).to_bytes());
    assert!(
        svc.cast_vote(proposal.id, founder.id, true, Some(&nay_sig))
            .await
            .is_err(),
        "a signature is bound to the exact decision"
    );

    // The genuine signature over the exact action → accepted.
    svc.cast_vote(proposal.id, founder.id, true, Some(&good_sig))
        .await
        .expect("a correctly-signed vote is accepted");
    assert!(svc.votes.has_voted(proposal.id, founder.id).await.unwrap());
}

#[tokio::test]
async fn a_keyless_account_can_still_vote_unsigned_during_rollout() {
    let svc = build();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let proposal = svc
        .open_proposal(
            founder.id,
            demos.id,
            ProposalKind::AddRule {
                text: "be kind".into(),
            },
        )
        .await
        .unwrap();

    // No key enrolled → the node-trusted fallback still applies.
    svc.cast_vote(proposal.id, founder.id, true, None)
        .await
        .expect("a keyless account votes unsigned during rollout");
    assert!(svc.votes.has_voted(proposal.id, founder.id).await.unwrap());
}

#[tokio::test]
async fn an_enrolled_account_must_sign_its_post_votes() {
    let svc = build();
    let founder = svc.register_user("founder").await.unwrap();
    let demos = svc
        .found_demos(founder.id, "rust", "Rustaceans")
        .await
        .unwrap();
    let post = svc
        .create_post(founder.id, demos.id, "hi", "body", vec![], vec![])
        .await
        .unwrap();

    let (sk, pub_hex) = user_key(7);
    svc.enroll_public_key(founder.id, &pub_hex).await.unwrap();

    // The client signs the *resolved* direction it is applying (here, an upvote).
    let msg = identity::post_vote_message::post_vote_message(post.id.0, Some(true));
    let good = hex(&sk.sign(msg.as_bytes()).to_bytes());

    assert!(
        svc.vote_post(post.id, founder.id, Some(true), None)
            .await
            .is_err(),
        "an enrolled account cannot post-vote unsigned"
    );
    // A signature over a *different* direction (downvote) can't authorize an upvote.
    let down = hex(&sk
        .sign(identity::post_vote_message::post_vote_message(post.id.0, Some(false)).as_bytes())
        .to_bytes());
    assert!(
        svc.vote_post(post.id, founder.id, Some(true), Some(&down))
            .await
            .is_err(),
        "the signature is bound to the exact direction"
    );
    let score = svc
        .vote_post(post.id, founder.id, Some(true), Some(&good))
        .await
        .expect("a correctly-signed post vote is accepted");
    assert_eq!(score, 1);
}

#[tokio::test]
async fn a_key_cannot_be_silently_replaced() {
    let svc = build();
    let user = svc.register_user("alice").await.unwrap();
    let (_, first) = user_key(3);
    let (_, second) = user_key(4);
    svc.enroll_public_key(user.id, &first).await.unwrap();
    assert!(
        svc.enroll_public_key(user.id, &second).await.is_err(),
        "re-enrolling a different key must be refused (would enable account takeover)"
    );
}

#[tokio::test]
async fn a_malformed_key_is_refused_at_enrollment() {
    let svc = build();
    let user = svc.register_user("alice").await.unwrap();
    assert!(svc.enroll_public_key(user.id, "not-a-key").await.is_err());
}
