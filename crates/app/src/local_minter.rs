//! The single-box / issuer-node implementation of [`AccountMinter`]: mint locally.

use async_trait::async_trait;

use domain::UserId;

use crate::{AccountMinter, MintAccountError, RegisterAccountError, Services};

/// Mints accounts against the local [`Services`] — correct when this process is
/// itself a trusted issuer, or when federation is off (single-box, where the one
/// node is the only issuer). The federated router falls back to this whenever it
/// finds it is running on a trusted-issuer node.
pub struct LocalMinter {
    services: Services,
}

impl LocalMinter {
    pub fn new(services: Services) -> Self {
        Self { services }
    }
}

/// Map a local registration failure onto the port's error. A store failure is
/// `Unavailable` (retryable); everything else is a merits `Rejected`.
fn to_mint_error(e: RegisterAccountError) -> MintAccountError {
    match e {
        RegisterAccountError::Store(s) => MintAccountError::Unavailable(s.to_string()),
        other => MintAccountError::Rejected(other.to_string()),
    }
}

#[async_trait]
impl AccountMinter for LocalMinter {
    async fn mint_account(
        &self,
        handle: &str,
        email: &str,
        password: &str,
    ) -> Result<UserId, MintAccountError> {
        self.services
            .register_account(handle, email, password)
            .await
            .map(|u| u.id)
            .map_err(to_mint_error)
    }
}
