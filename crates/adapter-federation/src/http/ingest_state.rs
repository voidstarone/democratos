use crate::Replicator;

/// What the ingest endpoint needs: the replicator that authorizes + applies.
#[derive(Clone)]
pub struct IngestState {
    pub replicator: std::sync::Arc<Replicator>,
    pub token: Option<String>,
}
