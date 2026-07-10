use std::collections::HashMap;

use app::Services;
use domain::NodeId;
use federation::OwnershipRegistry;

use crate::command::demos_of::demos_of;
use crate::command::execute::execute;
use crate::{changes_since, Command, CommandOutcome, ForwardError, IngestClient};

/// Executes a vote on the owner and **synchronously replicates it to a standby
/// before acking** — a quorum of 2.
///
/// After the use-case commits locally, the events it produced are drained from
/// the outbox, signed, and pushed to a designated standby; the call succeeds only
/// once a standby has applied them. If none acks, it **fails closed**: the caller
/// is told the vote was not durably recorded, rather than being led to believe a
/// single-node write is safe.
///
/// Note on durability: the vote does commit on the owner before the push, so the
/// hard guarantee of *zero* loss on an owner crash in the push window also wants
/// the standby configured as a Postgres synchronous physical replica (a
/// deployment concern). At the application layer this delivers 2-node logical
/// durability and read-your-writes on the standby in every non-crash case.
pub struct SyncVoteExecutor {
    node: NodeId,
    services: Services,
    store: std::sync::Arc<adapter_store_postgres::PostgresStore>,
    keypair: std::sync::Arc<federation::NodeKeypair>,
    registry: std::sync::Arc<dyn OwnershipRegistry>,
    /// node id → how to push to that standby.
    standbys: HashMap<u16, IngestClient>,
}

impl SyncVoteExecutor {
    pub fn new(
        node: NodeId,
        services: Services,
        store: std::sync::Arc<adapter_store_postgres::PostgresStore>,
        keypair: std::sync::Arc<federation::NodeKeypair>,
        registry: std::sync::Arc<dyn OwnershipRegistry>,
    ) -> Self {
        Self {
            node,
            services,
            store,
            keypair,
            registry,
            standbys: HashMap::new(),
        }
    }

    /// Register how to reach a standby node.
    pub fn with_standby(mut self, node: NodeId, client: IngestClient) -> Self {
        self.standbys.insert(node.0, client);
        self
    }

    /// Cast a vote with quorum-of-2 durability. Assumes this node owns the target
    /// community (the [`WriteRouter`](crate::WriteRouter) routes here only when local).
    pub async fn cast(&self, cmd: &Command) -> Result<CommandOutcome, ForwardError> {
        let demos = demos_of(&self.services, cmd)
            .await
            .map_err(ForwardError::App)?;
        let standbys = self
            .registry
            .standbys(demos.0)
            .await
            .map_err(|e| ForwardError::OwnerUnreachable(e.0))?;
        if standbys.is_empty() {
            return Err(ForwardError::OwnerUnreachable(
                "no standby designated — quorum of 2 impossible".into(),
            ));
        }

        // Execute authoritatively (commits locally, emits outbox events). A store
        // failure surfaces as its typed `StoreError` (ForwardError::App) and a
        // domain rejection as ForwardError::Rejected, so a caller behind the
        // `GovernanceWrites` port still sees "already voted" etc. (as a typed
        // `StoreError`) rather than a stringified message.
        let head = self.store.outbox_head().await.map_err(ForwardError::App)?;
        let outcome = execute(&self.services, cmd).await?;

        // Sign exactly the events this write produced and push to a standby.
        let events = changes_since(
            &self.store,
            &self.keypair,
            self.registry.as_ref(),
            head,
            1000,
        )
        .await
        .map_err(ForwardError::App)?;

        let mut last_err = String::from("no standby is designated");
        for sb in &standbys {
            if let Some(client) = self.standbys.get(&sb.0) {
                match client.push(self.node.0 as i64, &events).await {
                    Ok(_) => return Ok(outcome), // quorum of 2 reached
                    Err(e) => last_err = e.to_string(),
                }
            }
        }
        Err(ForwardError::OwnerUnreachable(format!(
            "vote committed locally but no standby acknowledged it (quorum of 2 not met): {last_err}"
        )))
    }
}
