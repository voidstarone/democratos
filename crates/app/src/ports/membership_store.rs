//! Persistence for community memberships.

use async_trait::async_trait;

use domain::{DemosId, Membership, Timestamp, UserId};

use crate::Result;

#[async_trait]
pub trait MembershipStore: Send + Sync {
    /// Insert or replace a membership (keyed by user + demos).
    async fn upsert(&self, membership: Membership) -> Result<()>;
    async fn get(&self, user: UserId, demos: DemosId) -> Result<Option<Membership>>;
    async fn members(&self, demos: DemosId) -> Result<Vec<Membership>>;
    /// Every community this user has joined. Backs the personalized home feed.
    async fn list_for_user(&self, user: UserId) -> Result<Vec<Membership>>;
    async fn voter_count(&self, demos: DemosId) -> Result<u64>;
    /// How many voters were enfranchised in this demos at or after `since`.
    /// Drives the Layer-2 rate cap.
    async fn admitted_since(&self, demos: DemosId, since: Timestamp) -> Result<u64>;
}
