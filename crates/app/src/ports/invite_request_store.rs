//! Persistence for the access waitlist (invite requests).

use async_trait::async_trait;

use domain::{InviteId, InviteRequest, Timestamp};

use crate::Result;

/// Stores the node-local invite waitlist. Not federated — a request lives only on
/// the node that took it. Adapters guarantee `email` is unique across live
/// requests (the service checks before creating, but a `UNIQUE` index is the real
/// backstop).
#[async_trait]
pub trait InviteRequestStore: Send + Sync {
    /// Record a new pending request. Callers normalize `email` first.
    async fn create(
        &self,
        email: &str,
        note: Option<&str>,
        requested_at: Timestamp,
    ) -> Result<InviteRequest>;
    async fn get(&self, id: InviteId) -> Result<Option<InviteRequest>>;
    /// Find a request by its (normalized) email — backs the idempotency check so a
    /// repeated ask doesn't pile up duplicate rows.
    async fn by_email(&self, email: &str) -> Result<Option<InviteRequest>>;
    /// Find a request by the SHA-256 hash of its one-time token — the accept-link
    /// lookup. Never store or query the raw token.
    async fn by_token_hash(&self, token_hash: &str) -> Result<Option<InviteRequest>>;
    /// Every request still awaiting a decision, oldest first — the review queue.
    async fn list_pending(&self) -> Result<Vec<InviteRequest>>;
    /// Approve a pending request: attach the issued token's hash and expiry.
    async fn approve(
        &self,
        id: InviteId,
        token_hash: &str,
        expires_at: Timestamp,
        decided_at: Timestamp,
    ) -> Result<()>;
    /// Reject a pending request.
    async fn reject(&self, id: InviteId, decided_at: Timestamp) -> Result<()>;
    /// Mark an approved request consumed once its account has been created — makes
    /// the token single-use.
    async fn mark_accepted(&self, id: InviteId) -> Result<()>;
}
