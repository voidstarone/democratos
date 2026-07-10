use std::sync::Arc;

use adapter_store_postgres::PostgresStore;
use federation::{NodeKeypair, OwnershipRegistry};

/// What the feed server needs to answer a pull.
#[derive(Clone)]
pub struct FeedState {
    pub store: Arc<PostgresStore>,
    pub keypair: Arc<NodeKeypair>,
    pub registry: Arc<dyn OwnershipRegistry>,
    /// Shared cluster bearer token; `None` disables the check (dev/local only).
    pub token: Option<String>,
}
