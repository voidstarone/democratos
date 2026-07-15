//! Personal blocking use-cases: a member hiding another member's content from
//! their own feeds and threads. Owns only the user port, so a consumer that just
//! needs the block relation doesn't depend on the whole app surface.

use std::sync::Arc;

use domain::UserId;

use crate::{Result, UserStore};

/// Personal blocking use-cases, over just the user store.
#[derive(Clone)]
pub struct BlockingService {
    users: Arc<dyn UserStore>,
}

impl BlockingService {
    pub fn new(users: Arc<dyn UserStore>) -> Self {
        Self { users }
    }

    /// Block `target` for `blocker`. Blocking yourself is a no-op. Idempotent.
    pub async fn block_user(&self, blocker: UserId, target: UserId) -> Result<()> {
        if blocker == target {
            return Ok(());
        }
        self.users.block_user(blocker, target).await
    }

    /// Lift `blocker`'s block on `target`. Idempotent.
    pub async fn unblock_user(&self, blocker: UserId, target: UserId) -> Result<()> {
        self.users.unblock_user(blocker, target).await
    }

    /// The accounts `viewer` has blocked. Empty if the account is gone. Feeds and
    /// threads filter their content against this set so a blocked author never
    /// reaches the viewer.
    pub async fn blocked_by(&self, viewer: UserId) -> Result<Vec<UserId>> {
        Ok(self
            .users
            .get(viewer)
            .await?
            .map(|u| u.blocked)
            .unwrap_or_default())
    }

    /// Whether `blocker` currently blocks `target`.
    pub async fn is_blocking(&self, blocker: UserId, target: UserId) -> Result<bool> {
        Ok(self
            .users
            .get(blocker)
            .await?
            .is_some_and(|u| u.blocks(target)))
    }
}
