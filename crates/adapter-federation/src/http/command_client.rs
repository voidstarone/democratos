use std::sync::Arc;

use federation::NodeKeypair;

use crate::command::signed_command::SignedCommand;
use crate::{Command, CommandOutcome, ForwardError};

/// Client for a peer node's command endpoint (the forwarding side).
pub struct CommandClient {
    base_url: String,
    token: Option<String>,
    /// This node's signing identity — every forwarded command is signed with it so
    /// the owner can authenticate which node produced it.
    keypair: Arc<NodeKeypair>,
    http: reqwest::Client,
}

impl CommandClient {
    pub fn new(
        base_url: impl Into<String>,
        token: Option<String>,
        keypair: Arc<NodeKeypair>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            token,
            keypair,
            http: reqwest::Client::new(),
        }
    }

    /// Forward a command to this peer and return its outcome. The command is
    /// Ed25519-signed with this node's key. Any transport failure surfaces as
    /// [`ForwardError::OwnerUnreachable`] (fail-closed); a domain rejection as
    /// [`ForwardError::Rejected`].
    pub async fn submit_command(&self, cmd: &Command) -> Result<CommandOutcome, ForwardError> {
        let url = format!("{}/federation/command", self.base_url.trim_end_matches('/'));
        let signed = SignedCommand::sign(&self.keypair, cmd);
        let mut req = self.http.post(url).json(&signed);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ForwardError::OwnerUnreachable(e.to_string()))?;
        let status = resp.status();
        if status.is_success() {
            resp.json()
                .await
                .map_err(|e| ForwardError::OwnerUnreachable(e.to_string()))
        } else if status.is_client_error() {
            let body = resp.text().await.unwrap_or_default();
            Err(ForwardError::Rejected(body))
        } else {
            Err(ForwardError::OwnerUnreachable(format!(
                "owner returned {status}"
            )))
        }
    }
}
