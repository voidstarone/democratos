//! The single-box / home-node implementation of [`AccountAuthenticator`].

use async_trait::async_trait;

use domain::UserId;

use crate::{AccountAuthenticator, AuthenticateError, Services};

/// Verifies logins against the local [`Services`] by handle — correct single-box, or
/// on the node that homes the account (which holds its credentials). The federated
/// authenticator falls back to this whenever the account's credentials are present
/// locally.
pub struct LocalAuthenticator {
    services: Services,
}

impl LocalAuthenticator {
    pub fn new(services: Services) -> Self {
        Self { services }
    }
}

#[async_trait]
impl AccountAuthenticator for LocalAuthenticator {
    async fn authenticate(
        &self,
        handle: &str,
        password: &str,
    ) -> Result<UserId, AuthenticateError> {
        self.services
            .authenticate_by_handle(handle, password)
            .await
            .map(|u| u.id)
    }
}
