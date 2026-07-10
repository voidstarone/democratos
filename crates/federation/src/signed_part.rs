//! The signed portion of a change event.

use serde::{Deserialize, Serialize};

use crate::ChangeOp;

/// The signed portion of a change event: everything a consumer's decision must
/// depend on. Serialized deterministically (serde emits a struct's fields in
/// declaration order) to produce the exact bytes that are signed and verified.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SignedPart {
    /// The node that produced (and signs) this event.
    pub node: u16,
    /// The ownership epoch the producer held when it emitted this. Fences a
    /// returning old owner: events under a stale epoch are rejected.
    pub epoch: u64,
    /// Monotonic per-node sequence (the producer's outbox id). The consumer's
    /// replay cursor.
    pub seq: u64,
    /// The community this change is scoped to, if any (`None` for global rows
    /// such as user accounts).
    pub demos: Option<u64>,
    /// The entity/table the row belongs to (e.g. `"posts"`, `"votes"`).
    pub entity: String,
    pub op: ChangeOp,
    /// The row itself, as JSON (`to_jsonb(row)` on the owner). For a delete this
    /// carries enough of the row to identify what to remove.
    pub payload: serde_json::Value,
}
