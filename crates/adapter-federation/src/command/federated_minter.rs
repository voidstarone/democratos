//! The federated [`AccountMinter`]: mint locally when this node is itself a trusted
//! issuer, otherwise discover a trusted issuer through the control plane and forward.

use std::sync::Arc;

use async_trait::async_trait;
use domain::{NodeId, UserId};
use federation::{choose_issuer, NodeKeypair, OwnershipRegistry};

use app::{AccountMinter, LocalMinter, MintAccountError};

use crate::http::command_client::CommandClient;
use crate::{Command, CommandOutcome, ForwardError};

/// Routes account minting. If this node holds a valid issuer cert it mints locally
/// (via [`LocalMinter`]); otherwise it asks the control plane for the trusted
/// issuers, picks the least-loaded reachable one ([`choose_issuer`]), and forwards a
/// signed [`Command::MintAccount`]. Fails closed when no issuer is available — an
/// un-certified node must never mint a fleet-wide account itself.
pub struct FederatedMinter {
    node: NodeId,
    local: LocalMinter,
    registry: Arc<dyn OwnershipRegistry>,
    keypair: Arc<NodeKeypair>,
    token: Option<String>,
}

impl FederatedMinter {
    pub fn new(
        node: NodeId,
        local: LocalMinter,
        registry: Arc<dyn OwnershipRegistry>,
        keypair: Arc<NodeKeypair>,
        token: Option<String>,
    ) -> Self {
        Self {
            node,
            local,
            registry,
            keypair,
            token,
        }
    }
}

/// Map a forwarded-mint failure onto the port's error: a merits refusal stays
/// `Rejected` (a 4xx the sign-up form shows); anything else is `Unavailable`.
fn forward_to_mint(e: ForwardError) -> MintAccountError {
    match e {
        ForwardError::Rejected(m) => MintAccountError::Rejected(m),
        ForwardError::Unowned => MintAccountError::NoIssuerAvailable,
        ForwardError::OwnerUnreachable(m) => MintAccountError::Unavailable(m),
        ForwardError::App(s) => MintAccountError::Unavailable(s.to_string()),
    }
}

#[async_trait]
impl AccountMinter for FederatedMinter {
    async fn mint_account(
        &self,
        handle: &str,
        email: &str,
        password: &str,
    ) -> Result<UserId, MintAccountError> {
        // This node is itself a trusted issuer → mint locally, no forwarding.
        let is_issuer = self
            .registry
            .is_trusted_issuer(self.node)
            .await
            .map_err(|e| MintAccountError::Unavailable(e.0))?;
        if is_issuer {
            // Claim the handle fleet-wide first (atomic), so this issuer and any other
            // can't both mint it. Release it if creation then fails.
            let reserved = self
                .registry
                .reserve_handle(handle.trim(), self.node)
                .await
                .map_err(|e| MintAccountError::Unavailable(e.0))?;
            if !reserved {
                return Err(MintAccountError::Rejected("that handle is taken".into()));
            }
            let result = self.local.mint_account(handle, email, password).await;
            if result.is_err() {
                let _ = self.registry.release_handle(handle.trim(), self.node).await;
            }
            return result;
        }

        // Otherwise discover the trusted issuers and forward to the least-loaded one.
        let issuers = self
            .registry
            .trusted_issuers()
            .await
            .map_err(|e| MintAccountError::Unavailable(e.0))?;
        let chosen = choose_issuer(&issuers).ok_or(MintAccountError::NoIssuerAvailable)?;

        let client = CommandClient::new(
            chosen.addr.clone(),
            self.token.clone(),
            self.keypair.clone(),
        );
        let cmd = Command::MintAccount {
            handle: handle.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        };
        match client.submit_command(&cmd).await.map_err(forward_to_mint)? {
            CommandOutcome::AccountMinted { id } => Ok(UserId(id)),
            other => Err(MintAccountError::Unavailable(format!(
                "issuer returned an unexpected outcome: {other:?}"
            ))),
        }
    }
}
