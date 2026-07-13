//! Verifying a login, whose authoritative home is the account's issuer.

use async_trait::async_trait;

use domain::UserId;

use crate::AuthenticateError;

/// Verifies a **handle + password** login and returns the account id.
///
/// Login is its own port for the same reason as [`AccountMinter`](crate::AccountMinter):
/// where it runs depends on trust. Credentials (email + hash) never replicate, so a
/// node that did not mint an account cannot verify its password locally — it must
/// ask the account's home issuer. On the home/single-box node this verifies locally;
/// elsewhere it forwards to the account's home issuer. Login is by handle because
/// handles replicate (emails are redacted from the feed). Failure is the opaque
/// [`AuthenticateError::InvalidCredentials`], so account existence never leaks.
#[async_trait]
pub trait AccountAuthenticator: Send + Sync {
    async fn authenticate(
        &self,
        handle: &str,
        password: &str,
    ) -> Result<UserId, AuthenticateError>;
}
