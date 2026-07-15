//! Facade delegators for personal blocking use-cases. The logic now lives in
//! [`BlockingService`](super::blocking_service::BlockingService); these thin
//! methods keep `services.block_user()` and friends working for call sites not
//! yet migrated off the `Services` aggregator.

use domain::UserId;

use crate::Result;

use super::blocking_service::BlockingService;
use super::services::Services;

impl Services {
    /// Build the extracted [`BlockingService`] from the ports this aggregator
    /// still holds. Cheap — `Arc` clones only — so delegators construct one per
    /// call rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `BlockingService` directly.
    pub(super) fn blocking_service(&self) -> BlockingService {
        BlockingService::new(self.users.clone())
    }

    /// Block `target` for `blocker`. Blocking yourself is a no-op. Idempotent.
    pub async fn block_user(&self, blocker: UserId, target: UserId) -> Result<()> {
        self.blocking_service().block_user(blocker, target).await
    }

    /// Lift `blocker`'s block on `target`. Idempotent.
    pub async fn unblock_user(&self, blocker: UserId, target: UserId) -> Result<()> {
        self.blocking_service().unblock_user(blocker, target).await
    }

    /// The accounts `viewer` has blocked. Empty if the account is gone. Feeds and
    /// threads filter their content against this set so a blocked author never
    /// reaches the viewer.
    pub async fn blocked_by(&self, viewer: UserId) -> Result<Vec<UserId>> {
        self.blocking_service().blocked_by(viewer).await
    }

    /// Whether `blocker` currently blocks `target`.
    pub async fn is_blocking(&self, blocker: UserId, target: UserId) -> Result<bool> {
        self.blocking_service().is_blocking(blocker, target).await
    }
}
