//! Facade delegators for invitation-only access use-cases. The logic now lives in
//! [`InviteService`](super::invite_service::InviteService); these thin methods
//! keep `services.request_invite()` and friends working for call sites not yet
//! migrated off the `Services` aggregator.

use domain::{InviteId, InviteRequest};

use crate::{AcceptInviteError, ApproveInviteError, RequestInviteError, Result};

use super::invite_service::InviteService;
use super::services::Services;

impl Services {
    /// Build the extracted [`InviteService`] from the ports this aggregator still
    /// holds. Cheap — `Arc` clones only — so delegators construct one per call
    /// rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `InviteService` directly.
    pub(super) fn invite_service(&self) -> InviteService {
        InviteService::new(
            self.invites.clone(),
            self.settings.clone(),
            self.notifier.clone(),
            self.users.clone(),
            self.public_base_url.clone(),
            self.invite_token_ttl_days,
            self.clock.clone(),
        )
    }

    /// Whether new sign-ups currently require an invite. Reads the persisted
    /// operator toggle, falling back to `default_when_unset` (the node's boot
    /// flag) when it has never been set. Cheap enough to call per request.
    pub async fn is_invite_only(&self, default_when_unset: bool) -> Result<bool> {
        self.invite_service().is_invite_only(default_when_unset).await
    }

    /// Turn invitation-only access on or off, persisting the choice so it survives
    /// a restart.
    pub async fn set_invite_only(&self, invite_only: bool) -> Result<()> {
        self.invite_service().set_invite_only(invite_only).await
    }

    /// Take a request for an account from the public waitlist form.
    ///
    /// Deliberately idempotent and enumeration-safe: a blank email is rejected,
    /// but an email that already has a live request — or already belongs to an
    /// account — quietly returns `Ok` without creating a second row and without
    /// revealing which case it was. So the public form can never be used to probe
    /// who is already registered or already waiting.
    pub async fn request_invite(
        &self,
        email: &str,
        note: Option<&str>,
    ) -> Result<(), RequestInviteError> {
        self.invite_service().request_invite(email, note).await
    }

    /// The review queue: every request still awaiting a decision, oldest first.
    pub async fn list_pending_invites(&self) -> Result<Vec<InviteRequest>> {
        self.invite_service().list_pending_invites().await
    }

    /// Approve a pending request: mint a one-time token, email the requester the
    /// accept link, and — only if the email is accepted for delivery — record the
    /// approval. The email goes out *before* the store is marked so a delivery
    /// failure leaves the request pending and retryable rather than approved yet
    /// unreachable.
    pub async fn approve_invite(&self, id: InviteId) -> Result<(), ApproveInviteError> {
        self.invite_service().approve_invite(id).await
    }

    /// Reject a pending request. No email is sent.
    pub async fn reject_invite(&self, id: InviteId) -> Result<(), ApproveInviteError> {
        self.invite_service().reject_invite(id).await
    }

    /// Resolve a raw invite token to its (still-redeemable) request, so the accept
    /// flow can bind the new account to the invited email. Returns the opaque
    /// [`AcceptInviteError::InvalidToken`] for an unknown, expired, or already-used
    /// token alike.
    pub async fn validate_invite_token(
        &self,
        token: &str,
    ) -> Result<InviteRequest, AcceptInviteError> {
        self.invite_service().validate_invite_token(token).await
    }

    /// Consume an approved invite once its account has been created — makes the
    /// token single-use.
    pub async fn mark_invite_accepted(&self, id: InviteId) -> Result<(), AcceptInviteError> {
        self.invite_service().mark_invite_accepted(id).await
    }
}
