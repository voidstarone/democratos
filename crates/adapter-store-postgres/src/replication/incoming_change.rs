//! One change to apply to the local replica.

use serde_json::Value;

/// One change to apply to the local replica, as carried by a verified event.
#[derive(Debug, Clone)]
pub struct IncomingChange {
    /// The producer's sequence — the ordering key and the replay cursor.
    pub seq: i64,
    /// The table the row belongs to.
    pub entity: String,
    /// `"upsert"` or `"delete"`.
    pub op: String,
    /// `to_jsonb(row)` from the producer.
    pub payload: Value,
}
