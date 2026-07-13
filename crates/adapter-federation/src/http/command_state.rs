use std::sync::Arc;

use app::Services;
use domain::NodeId;
use federation::OwnershipRegistry;

/// What the command endpoint needs to run a forwarded write authoritatively.
#[derive(Clone)]
pub struct CommandState {
    /// This (issuer/owner) node's id — e.g. the holder of a handle reservation made
    /// when minting a delegated account.
    pub node: NodeId,
    pub services: Services,
    pub token: Option<String>,
    /// Resolves the forwarding node's published key, so a command is authenticated
    /// to a specific node identity — not merely accompanied by the shared token.
    pub registry: Arc<dyn OwnershipRegistry>,
    /// Recently-seen command nonces, so a captured command can't be replayed and
    /// the same command can't be applied twice. Shared across requests.
    pub replay_guard: Arc<crate::command::replay_guard::ReplayGuard>,
    /// Per-requesting-node cap on delegated account minting, so an authenticated but
    /// abusive node can't flood the federation with accounts. Shared across requests.
    pub mint_rate_limiter: Arc<crate::command::mint_rate_limiter::MintRateLimiter>,
    /// Per-target-handle cap on delegated login verification — the brute-force guard
    /// on the delegated-auth path. Shared across requests.
    pub auth_rate_limiter: Arc<crate::command::auth_rate_limiter::AuthRateLimiter>,
}
