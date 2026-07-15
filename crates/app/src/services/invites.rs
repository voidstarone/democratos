

use domain::{
    InviteId, InviteRequest,
};


use crate::invite::hash_token::hash_token;
use crate::invite::new_invite_token::new_invite_token;
use crate::{
    AcceptInviteError, ApproveInviteError, RequestInviteError, Result,
};


use super::services::Services;

impl Services {
    /// Whether new sign-ups currently require an invite. Reads the persisted
    /// operator toggle, falling back to `default_when_unset` (the node's boot
    /// flag) when it has never been set. Cheap enough to call per request.
    pub async fn is_invite_only(&self, default_when_unset: bool) -> Result<bool> {
        Ok(self
            .settings
            .is_invite_only()
            .await?
            .unwrap_or(default_when_unset))
    }

    /// Turn invitation-only access on or off, persisting the choice so it survives
    /// a restart.
    pub async fn set_invite_only(&self, invite_only: bool) -> Result<()> {
        self.settings.set_invite_only(invite_only).await
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
        let email = domain::normalize_email(email);
        domain::validate_email(&email).map_err(|e| RequestInviteError::Rejected(e.message()))?;

        // Already an account, or already on the list → no-op, no leak.
        if self.users.by_email(&email).await?.is_some() {
            return Ok(());
        }
        if self.invites.by_email(&email).await?.is_some() {
            return Ok(());
        }

        let note = note.map(str::trim).filter(|n| !n.is_empty());
        self.invites
            .create(&email, note, self.clock.now())
            .await?;
        Ok(())
    }

    /// The review queue: every request still awaiting a decision, oldest first.
    pub async fn list_pending_invites(&self) -> Result<Vec<InviteRequest>> {
        self.invites.list_pending().await
    }

    /// Approve a pending request: mint a one-time token, email the requester the
    /// accept link, and — only if the email is accepted for delivery — record the
    /// approval. The email goes out *before* the store is marked so a delivery
    /// failure leaves the request pending and retryable rather than approved yet
    /// unreachable.
    pub async fn approve_invite(&self, id: InviteId) -> Result<(), ApproveInviteError> {
        let request = self
            .invites
            .get(id)
            .await?
            .ok_or(ApproveInviteError::NotPending)?;
        if request.status != domain::InviteStatus::Pending {
            return Err(ApproveInviteError::NotPending);
        }

        let token = new_invite_token();
        let accept_url = format!(
            "{}/invite/accept?token={}",
            self.public_base_url.trim_end_matches('/'),
            token
        );
        // Deliver first — if this fails, the request stays Pending.
        self.notifier
            .notify_invite_approved(&request.email, &accept_url)
            .await?;

        let now = self.clock.now();
        let expires_at = now.plus_days(self.invite_token_ttl_days);
        self.invites
            .approve(id, &hash_token(&token), expires_at, now)
            .await?;
        Ok(())
    }

    /// Reject a pending request. No email is sent.
    pub async fn reject_invite(&self, id: InviteId) -> Result<(), ApproveInviteError> {
        let request = self
            .invites
            .get(id)
            .await?
            .ok_or(ApproveInviteError::NotPending)?;
        if request.status != domain::InviteStatus::Pending {
            return Err(ApproveInviteError::NotPending);
        }
        self.invites.reject(id, self.clock.now()).await?;
        Ok(())
    }

    /// Resolve a raw invite token to its (still-redeemable) request, so the accept
    /// flow can bind the new account to the invited email. Returns the opaque
    /// [`AcceptInviteError::InvalidToken`] for an unknown, expired, or already-used
    /// token alike.
    pub async fn validate_invite_token(
        &self,
        token: &str,
    ) -> Result<InviteRequest, AcceptInviteError> {
        let request = self
            .invites
            .by_token_hash(&hash_token(token))
            .await?
            .ok_or(AcceptInviteError::InvalidToken)?;
        if !request.is_redeemable(self.clock.now()) {
            return Err(AcceptInviteError::InvalidToken);
        }
        Ok(request)
    }

    /// Consume an approved invite once its account has been created — makes the
    /// token single-use.
    pub async fn mark_invite_accepted(&self, id: InviteId) -> Result<(), AcceptInviteError> {
        self.invites.mark_accepted(id).await?;
        Ok(())
    }
}
