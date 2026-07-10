use app::Services;
use domain::NodeId;
use federation::OwnershipRegistry;

use crate::command::demos_of::demos_of;
use crate::command::execute::execute;
use crate::{Command, CommandOutcome, CommandTransport, ForwardError, SyncVoteExecutor};

/// Routes writes: execute locally when this node owns the target community, else
/// forward to the owner. Fail-closed when there is no reachable owner.
pub struct WriteRouter {
    node: NodeId,
    services: Services,
    registry: std::sync::Arc<dyn OwnershipRegistry>,
    transport: std::sync::Arc<dyn CommandTransport>,
    /// When set, a write this node owns is executed with quorum-of-2 durability
    /// instead of a bare local commit. Off → a plain local `execute`.
    sync: Option<std::sync::Arc<SyncVoteExecutor>>,
}

impl WriteRouter {
    pub fn new(
        node: NodeId,
        services: Services,
        registry: std::sync::Arc<dyn OwnershipRegistry>,
        transport: std::sync::Arc<dyn CommandTransport>,
    ) -> Self {
        Self {
            node,
            services,
            registry,
            transport,
            sync: None,
        }
    }

    /// Execute locally-owned writes through `sync` (sync-replicate to a standby
    /// before acking; fail-closed if none). Without this, a locally-owned write
    /// commits without waiting for a standby.
    pub fn with_sync(mut self, sync: std::sync::Arc<SyncVoteExecutor>) -> Self {
        self.sync = Some(sync);
        self
    }

    /// Submit a write. Resolves the target community's owner and either runs the
    /// use-case locally (we own it) or forwards it (someone else does). Never
    /// applies a non-owned write locally.
    pub async fn submit(&self, cmd: Command) -> Result<CommandOutcome, ForwardError> {
        let demos = demos_of(&self.services, &cmd)
            .await
            .map_err(ForwardError::App)?;
        let owner = self
            .registry
            .owner_of(demos.0)
            .await
            .map_err(|e| ForwardError::OwnerUnreachable(e.0))?
            .ok_or(ForwardError::Unowned)?;
        if owner.owner == self.node {
            match &self.sync {
                Some(sync) => sync.cast(&cmd).await,
                None => execute(&self.services, &cmd).await,
            }
        } else {
            self.transport.send(owner.owner, &cmd).await
        }
    }
}
