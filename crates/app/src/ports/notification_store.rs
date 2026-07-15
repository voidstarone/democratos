//! Persistence for in-app notifications (node-local; never federated).

use async_trait::async_trait;

use domain::{Notification, NotificationKind, Timestamp, UserId};

use crate::Result;

#[async_trait]
pub trait NotificationStore: Send + Sync {
    /// Record a notification for `recipient` and return it (id assigned by the
    /// store). Callers gate on the recipient's per-kind preference *before*
    /// pushing, so anything stored is something the recipient opted to receive.
    async fn push(
        &self,
        recipient: UserId,
        kind: NotificationKind,
        at: Timestamp,
    ) -> Result<Notification>;
    /// This recipient's notifications, newest first, capped to a recent window.
    async fn list_for(&self, recipient: UserId) -> Result<Vec<Notification>>;
    /// How many of this recipient's notifications are still unseen — the number
    /// the toolbar badge shows.
    async fn unread_count(&self, recipient: UserId) -> Result<u64>;
    /// Mark every one of this recipient's notifications seen (called when they
    /// open their notifications), clearing the badge.
    async fn mark_all_seen(&self, recipient: UserId) -> Result<()>;
}
