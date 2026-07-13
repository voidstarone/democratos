//! Invitation-only flow through the `Services` facade: request → approve (emails
//! a one-time link) → accept (mints the account, consumes the token), plus the
//! reject path, single-use enforcement, expiry, and the persisted toggle.

use std::sync::Arc;

use adapter_moderation_local::{AutoApproveAgeVerifier, HeuristicNsfwScanner};
use adapter_notify::RecordingNotifier;
use adapter_recommend_memory::MemoryRecommender;
use adapter_store_memory::{FixedClock, MemoryStore};
use app::{AcceptInviteError, ApproveInviteError, Services};
use domain::{InviteId, Timestamp};

/// Build `Services` over the in-memory store, handing back the recording notifier
/// so a test can inspect what invite emails would have gone out. Time is fixed and
/// advanced explicitly via the returned clock.
fn build() -> (Services, Arc<RecordingNotifier>, Arc<FixedClock>) {
    let store = Arc::new(MemoryStore::new());
    let clock = Arc::new(FixedClock::at(Timestamp(1_000)));
    let notifier = Arc::new(RecordingNotifier::new());
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
        invites: store.clone(),
        settings: store.clone(),
        sensitive_cases: store.clone(),
        trials: store.clone(),
        post_votes: store.clone(),
        comment_votes: store.clone(),
        media: store,
        recommender: Arc::new(MemoryRecommender::default()),
        nsfw_scanner: Arc::new(HeuristicNsfwScanner),
        age_verifier: Arc::new(AutoApproveAgeVerifier),
        requires_age_verification: false,
        require_signatures: false,
        notifier: notifier.clone(),
        public_base_url: "https://demos.example".to_string(),
        invite_token_ttl_days: 7,
        clock: clock.clone(),
    };
    (services, notifier, clock)
}

/// The token embedded in the accept link the notifier was handed.
fn token_from_last_email(notifier: &RecordingNotifier) -> String {
    let (_, url) = notifier.sent().last().expect("an email was sent").clone();
    url.split("token=")
        .nth(1)
        .expect("accept url carries a token")
        .to_string()
}

#[tokio::test]
async fn request_approve_accept_creates_the_account() {
    let (svc, notifier, _clock) = build();

    svc.request_invite("Alice@Example.com", Some(" wants in "))
        .await
        .unwrap();

    // One pending request, email normalized, note trimmed.
    let pending = svc.list_pending_invites().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].email, "alice@example.com");
    assert_eq!(pending[0].note.as_deref(), Some("wants in"));

    // Approve → an email with a one-time link goes out to the invited address.
    svc.approve_invite(pending[0].id).await.unwrap();
    let sent = notifier.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "alice@example.com");
    assert!(sent[0].1.starts_with("https://demos.example/invite/accept?token="));

    // No longer pending once approved.
    assert!(svc.list_pending_invites().await.unwrap().is_empty());

    // The token resolves to the invited email.
    let token = token_from_last_email(&notifier);
    let bound = svc.validate_invite_token(&token).await.unwrap();
    assert_eq!(bound.email, "alice@example.com");

    // Accepting mints the account bound to that email and consumes the invite.
    let user = svc
        .register_account("alice", &bound.email, "correct horse battery")
        .await
        .unwrap();
    svc.mark_invite_accepted(bound.id).await.unwrap();
    assert_eq!(user.email.as_deref(), Some("alice@example.com"));

    // The token is now single-use — a replay is rejected.
    assert!(matches!(
        svc.validate_invite_token(&token).await,
        Err(AcceptInviteError::InvalidToken)
    ));
}

#[tokio::test]
async fn request_is_idempotent_and_enumeration_safe() {
    let (svc, _notifier, _clock) = build();

    svc.request_invite("bob@example.com", None).await.unwrap();
    // A second ask for the same email must not create a second row...
    svc.request_invite("bob@example.com", None).await.unwrap();
    assert_eq!(svc.list_pending_invites().await.unwrap().len(), 1);

    // ...and an email that already has an account is a silent no-op, not an error
    // and not a new waitlist row (so the form can't probe who's registered).
    svc.register_account("carol", "carol@example.com", "correct horse battery")
        .await
        .unwrap();
    svc.request_invite("carol@example.com", None).await.unwrap();
    assert_eq!(svc.list_pending_invites().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_rejected_request_leaves_the_queue_and_issues_no_token() {
    let (svc, notifier, _clock) = build();
    svc.request_invite("dave@example.com", None).await.unwrap();
    let id = svc.list_pending_invites().await.unwrap()[0].id;

    svc.reject_invite(id).await.unwrap();
    assert!(svc.list_pending_invites().await.unwrap().is_empty());
    assert!(notifier.sent().is_empty(), "reject sends no email");

    // Re-approving a decided request is refused.
    assert!(matches!(
        svc.approve_invite(id).await,
        Err(ApproveInviteError::NotPending)
    ));
}

#[tokio::test]
async fn an_expired_token_is_rejected() {
    let (svc, notifier, clock) = build();
    svc.request_invite("erin@example.com", None).await.unwrap();
    let id = svc.list_pending_invites().await.unwrap()[0].id;
    svc.approve_invite(id).await.unwrap();
    let token = token_from_last_email(&notifier);

    // Still valid a day later...
    clock.set(Timestamp(1_000 + 86_400));
    assert!(svc.validate_invite_token(&token).await.is_ok());

    // ...but not past the 7-day TTL.
    clock.set(Timestamp(1_000 + 8 * 86_400));
    assert!(matches!(
        svc.validate_invite_token(&token).await,
        Err(AcceptInviteError::InvalidToken)
    ));
}

#[tokio::test]
async fn approving_an_unknown_id_is_not_pending() {
    let (svc, _notifier, _clock) = build();
    assert!(matches!(
        svc.approve_invite(InviteId(999)).await,
        Err(ApproveInviteError::NotPending)
    ));
}

#[tokio::test]
async fn invite_only_toggle_persists() {
    let (svc, _notifier, _clock) = build();
    // Unset → falls back to the supplied boot default.
    assert!(!svc.is_invite_only(false).await.unwrap());
    assert!(svc.is_invite_only(true).await.unwrap());

    // Once set, the persisted value wins over the default.
    svc.set_invite_only(true).await.unwrap();
    assert!(svc.is_invite_only(false).await.unwrap());
    svc.set_invite_only(false).await.unwrap();
    assert!(!svc.is_invite_only(true).await.unwrap());
}
