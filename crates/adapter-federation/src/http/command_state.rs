use std::sync::Arc;

use app::Services;
use federation::OwnershipRegistry;

/// What the command endpoint needs to run a forwarded write authoritatively.
#[derive(Clone)]
pub struct CommandState {
    pub services: Services,
    pub token: Option<String>,
    /// Resolves the forwarding node's published key, so a command is authenticated
    /// to a specific node identity — not merely accompanied by the shared token.
    pub registry: Arc<dyn OwnershipRegistry>,
    /// Recently-seen command nonces, so a captured command can't be replayed and
    /// the same command can't be applied twice. Shared across requests.
    pub replay_guard: Arc<crate::command::replay_guard::ReplayGuard>,
}
