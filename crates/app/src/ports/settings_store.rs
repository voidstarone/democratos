//! Persistence for node-operator settings that must survive a restart.

use async_trait::async_trait;

use crate::Result;

/// A tiny key–value corner for operator-toggleable node settings. Today it holds
/// only the invitation-only switch, so the operator can flip access on or off from
/// the admin console and have the choice stick across restarts (a boot flag only
/// seeds the initial value). Node-local, like the waitlist it guards.
#[async_trait]
pub trait SettingsStore: Send + Sync {
    /// The persisted invitation-only toggle, or `None` if it has never been set —
    /// in which case the node keeps its boot default.
    async fn is_invite_only(&self) -> Result<Option<bool>>;
    /// Persist the invitation-only toggle.
    async fn set_invite_only(&self, invite_only: bool) -> Result<()>;
}
