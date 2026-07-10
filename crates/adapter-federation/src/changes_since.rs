use adapter_store_postgres::PostgresStore;
use app::Result;
use federation::{
    event_scope, ChangeEvent, ChangeOp, EventScope, NodeKeypair, OwnershipRegistry, SignedPart,
};

fn parse_op(op: &str) -> ChangeOp {
    match op {
        "delete" => ChangeOp::Delete,
        _ => ChangeOp::Upsert,
    }
}

/// Produce this node's change feed after `after_seq`, each event **signed** and
/// stamped with the current ownership epoch for its community (so a consumer can
/// fence a feed produced under a stale epoch). Only communities this node
/// currently owns get a real epoch; global rows (e.g. users) use epoch 0.
pub async fn changes_since(
    store: &PostgresStore,
    keypair: &NodeKeypair,
    registry: &dyn OwnershipRegistry,
    after_seq: i64,
    limit: i64,
) -> Result<Vec<ChangeEvent>> {
    let recs = store.outbox_since(after_seq, limit).await?;
    let mut out = Vec::with_capacity(recs.len());
    // Cache each community's epoch for the duration of this call. A batch (or a
    // synchronous vote push draining many rows) is dominated by one community, so
    // this turns O(events) control-plane look-ups into O(distinct communities).
    let mut epoch_cache: std::collections::HashMap<i64, u64> = std::collections::HashMap::new();
    for rec in recs {
        let part = SignedPart {
            node: 0,  // stamped by sign()
            epoch: 0, // set below, once the scoping community is known
            seq: rec.seq as u64,
            demos: rec.demos.map(|d| d as u64),
            entity: rec.entity,
            op: parse_op(&rec.op),
            payload: rec.payload,
        };
        // Stamp the epoch of the community the row *actually* belongs to, matching
        // how the consumer derives it in `authorize`. For the `demoi` community
        // row this is its own id (not the outbox's NULL `demos_id`); for ballots
        // it is the parent-scoped `demos_id` the capture trigger already recorded.
        let epoch_demos: Option<i64> = match event_scope(&part) {
            EventScope::Demos(d) => Some(d as i64),
            EventScope::ViaParent { .. } => rec.demos,
            EventScope::Global { .. } | EventScope::Indeterminate => None,
        };
        let epoch = match epoch_demos {
            Some(d) => match epoch_cache.get(&d) {
                Some(&e) => e,
                None => {
                    let e = registry
                        .owner_of(d as u64)
                        .await
                        .ok()
                        .flatten()
                        .map(|o| o.epoch)
                        .unwrap_or(0);
                    epoch_cache.insert(d, e);
                    e
                }
            },
            None => 0,
        };
        let part = SignedPart { epoch, ..part };
        out.push(ChangeEvent::sign(keypair, part));
    }
    Ok(out)
}
