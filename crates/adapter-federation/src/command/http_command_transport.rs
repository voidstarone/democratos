use std::collections::HashMap;

use async_trait::async_trait;

use domain::NodeId;

use crate::{Command, CommandClient, CommandOutcome, CommandTransport, ForwardError};

/// HTTP implementation of [`CommandTransport`]: one client per known peer node.
#[derive(Default)]
pub struct HttpCommandTransport {
    peers: HashMap<u16, CommandClient>,
}

impl HttpCommandTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register how to reach `node`'s command endpoint.
    pub fn with_peer(mut self, node: NodeId, client: CommandClient) -> Self {
        self.peers.insert(node.0, client);
        self
    }
}

#[async_trait]
impl CommandTransport for HttpCommandTransport {
    async fn send(&self, owner: NodeId, cmd: &Command) -> Result<CommandOutcome, ForwardError> {
        let client = self
            .peers
            .get(&owner.0)
            .ok_or_else(|| ForwardError::OwnerUnreachable(format!("no route to node {owner}")))?;
        client.submit_command(cmd).await
    }
}
