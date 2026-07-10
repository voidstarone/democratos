use async_trait::async_trait;

use domain::NodeId;

use crate::{Command, CommandOutcome, ForwardError};

/// How a router reaches a remote owner. Abstracted so the routing logic is
/// testable without a network (the HTTP implementation is
/// [`HttpCommandTransport`](crate::HttpCommandTransport)).
#[async_trait]
pub trait CommandTransport: Send + Sync {
    async fn send(&self, owner: NodeId, cmd: &Command) -> Result<CommandOutcome, ForwardError>;
}
