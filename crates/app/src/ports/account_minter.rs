//! The account-**minting** operation, whose authoritative home is a trusted issuer.

use async_trait::async_trait;

use domain::UserId;

use crate::MintAccountError;

/// Creates a real (email + password) account and returns its id.
///
/// Account creation is not community-scoped, so it does not go through
/// [`GovernanceWrites`](crate::GovernanceWrites); it has its own port because *where*
/// it runs differs by trust. On a trusted-issuer node it mints locally; on any other
/// node it is forwarded to a trusted issuer that mints in *its* id namespace, so the
/// account replicates fleet-wide (an un-certified node's own account would be
/// rejected everywhere). The delivery adapter (web sign-up) depends only on this
/// port; the composition root plugs in the local or federated implementation.
#[async_trait]
pub trait AccountMinter: Send + Sync {
    async fn mint_account(
        &self,
        handle: &str,
        email: &str,
        password: &str,
    ) -> Result<UserId, MintAccountError>;
}
