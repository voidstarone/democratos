//! The federated [`AccountAuthenticator`]: verify locally when this node holds the
//! account's credentials, otherwise forward to the account's **home** issuer.

use std::sync::Arc;

use async_trait::async_trait;
use domain::{origin_node, NodeId, UserId};
use federation::{NodeKeypair, OwnershipRegistry};

use app::{AccountAuthenticator, AuthenticateError, Services, StoreError};

use crate::http::command_client::CommandClient;
use crate::{Command, CommandOutcome, ForwardError};

/// Routes login verification. If the account's credentials live on this node (it
/// minted the account, or is single-box) it verifies locally; otherwise it resolves
/// the account's home issuer from the id (`origin_node`), looks up that node's
/// address in the control plane, and forwards a signed [`Command::Authenticate`].
/// Login is by handle because handles replicate and emails do not.
pub struct FederatedAuthenticator {
    node: NodeId,
    services: Services,
    registry: Arc<dyn OwnershipRegistry>,
    keypair: Arc<NodeKeypair>,
    token: Option<String>,
}

impl FederatedAuthenticator {
    pub fn new(
        node: NodeId,
        services: Services,
        registry: Arc<dyn OwnershipRegistry>,
        keypair: Arc<NodeKeypair>,
        token: Option<String>,
    ) -> Self {
        Self {
            node,
            services,
            registry,
            keypair,
            token,
        }
    }
}

/// Map a forwarded-auth failure onto the login error. A merits refusal from the
/// issuer collapses to the opaque `InvalidCredentials` (so it is indistinguishable
/// from a local wrong password); anything else is an availability failure.
fn forward_to_auth(e: ForwardError) -> AuthenticateError {
    match e {
        ForwardError::Rejected(_) => AuthenticateError::InvalidCredentials,
        ForwardError::App(s) => AuthenticateError::Store(s),
        ForwardError::Unowned | ForwardError::OwnerUnreachable(_) => {
            AuthenticateError::Store(StoreError::Store(
                "the account's home server is currently unreachable".into(),
            ))
        }
    }
}

#[async_trait]
impl AccountAuthenticator for FederatedAuthenticator {
    async fn authenticate(
        &self,
        handle: &str,
        password: &str,
    ) -> Result<UserId, AuthenticateError> {
        // Resolve the account locally (its row replicates even though its credentials
        // don't). If we can't see it, treat it as a normal failed login — opaque, so
        // handle enumeration across the *credentialed* set leaks nothing extra.
        let Some(user) = self.services.users.by_handle(handle.trim()).await? else {
            return self
                .services
                .authenticate_by_handle(handle, password)
                .await
                .map(|u| u.id);
        };

        let home = origin_node(user.id.0);
        // We hold the credentials (we minted it, or single-box) → verify locally.
        // Also the fallback when we ARE the home node: local verify is authoritative.
        if user.password_hash.is_some() || home == self.node {
            return self
                .services
                .authenticate_by_handle(handle, password)
                .await
                .map(|u| u.id);
        }

        // Otherwise forward to the home issuer, whose address is in the control plane.
        let addr = self
            .registry
            .node_addr(home)
            .await
            .map_err(|e| AuthenticateError::Store(StoreError::Store(e.0)))?
            .ok_or_else(|| {
                AuthenticateError::Store(StoreError::Store(
                    "the account's home server is not reachable".into(),
                ))
            })?;

        let client = CommandClient::new(addr, self.token.clone(), self.keypair.clone());
        let cmd = Command::Authenticate {
            handle: handle.to_string(),
            password: password.to_string(),
        };
        match client.submit_command(&cmd).await.map_err(forward_to_auth)? {
            CommandOutcome::Authenticated { id } => Ok(UserId(id)),
            _ => Err(AuthenticateError::InvalidCredentials),
        }
    }
}
