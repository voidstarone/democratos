//! One row read from this node's outbox.

use serde_json::Value;

/// One row read from this node's outbox, ready to be signed and served.
#[derive(Debug, Clone)]
pub struct OutboxRecord {
    /// This node's monotonic event sequence — the consumer's replay cursor.
    pub seq: i64,
    /// The table the row belongs to.
    pub entity: String,
    /// `"upsert"` or `"delete"`.
    pub op: String,
    /// Scoping community, if any.
    pub demos: Option<i64>,
    /// `to_jsonb(row)` captured by the outbox trigger.
    pub payload: Value,
}
