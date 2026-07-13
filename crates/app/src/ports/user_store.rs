//! Persistence for user accounts.

use async_trait::async_trait;

use domain::{FeedPaging, Timestamp, User, UserId};

use crate::Result;

#[async_trait]
pub trait UserStore: Send + Sync {
    /// Create an account. `email`/`password_hash` are `None` for the dev-only
    /// handle switcher and `Some` for a real sign-up; the store persists them
    /// verbatim (the hash is already opaque). Callers guarantee handle — and
    /// email, when present — are unique.
    async fn create(
        &self,
        handle: &str,
        email: Option<&str>,
        password_hash: Option<&str>,
        created_at: Timestamp,
    ) -> Result<User>;
    async fn get(&self, id: UserId) -> Result<Option<User>>;
    async fn by_handle(&self, handle: &str) -> Result<Option<User>>;
    /// Look up an account by its (normalized) login email. Backs the uniqueness
    /// check on sign-up and the lookup on sign-in.
    async fn by_email(&self, email: &str) -> Result<Option<User>>;
    /// All registered users, oldest first. Used by dev tooling to enumerate
    /// accounts; production flows look users up by id or handle.
    async fn list(&self) -> Result<Vec<User>>;
    /// Record the outcome of age verification for a user.
    async fn set_is_age_verified(&self, id: UserId, is_verified: bool) -> Result<()>;
    /// Persist the account's enrolled Ed25519 public signing key (hex). Callers
    /// enforce the enrolment policy (e.g. first-key-only); the store just stores it.
    async fn set_public_key(&self, id: UserId, public_key_hex: &str) -> Result<()>;
    /// Persist whether the account is permanently barred from the franchise (the
    /// dev/content puppet flag). The store just records it; the domain enforces the
    /// bar. Replicates like any other user field.
    async fn set_franchise_barred(&self, id: UserId, barred: bool) -> Result<()>;
    /// Persist the account's feed-delivery preference (paged vs. lazy-loaded).
    async fn set_feed_paging(&self, id: UserId, paging: FeedPaging) -> Result<()>;
    /// Opt the account in to (or out of) reviewing platform-wide sensitive content.
    /// Default off; deliberately not a demos tier.
    async fn set_sensitive_reviewer(&self, id: UserId, is_reviewer: bool) -> Result<()>;
}
