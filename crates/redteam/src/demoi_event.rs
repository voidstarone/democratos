//! Build a signed `demoi` (community config) rewrite event.

use federation::{ChangeEvent, ChangeOp, NodeKeypair, SignedPart};

/// A `demoi` (community config) upsert row — `EventScope::Demos(id)`, so it needs no
/// parent resolution. Rewriting it is a vivid "attacker renamed the community" probe.
pub(crate) fn demoi_event(
    kp: &NodeKeypair,
    demos: u64,
    epoch: u64,
    seq: u64,
    name: &str,
) -> ChangeEvent {
    ChangeEvent::sign(
        kp,
        SignedPart {
            node: 0, // stamped by sign()
            epoch,
            seq,
            demos: Some(demos),
            entity: "demoi".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "id": demos, "slug": "pwned", "name": name }),
        },
    )
}
