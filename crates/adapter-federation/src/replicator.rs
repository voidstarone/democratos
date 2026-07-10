use std::sync::Arc;

use adapter_store_postgres::{IncomingChange, PostgresStore};
use app::Result;
use federation::{
    authorize, AuthError, ChangeEvent, ChangeOp, OwnershipRegistry, ParentDemos, ParentKind,
    RegistryError,
};

use crate::Ingested;

/// Resolves a ballot's community from its parent row on the local Postgres
/// replica. This is the data-access half of [`federation::authorize`] — the pure
/// crate classifies an event as parent-scoped; this looks the parent up.
struct StoreParents(Arc<PostgresStore>);

#[async_trait::async_trait]
impl ParentDemos for StoreParents {
    async fn parent_demos(
        &self,
        kind: ParentKind,
        id: u64,
    ) -> std::result::Result<Option<u64>, RegistryError> {
        let table = match kind {
            ParentKind::Proposal => "proposals",
            ParentKind::Post => "posts",
            ParentKind::Trial => "trials",
        };
        self.0
            .parent_demos(table, id as i64)
            .await
            .map(|opt| opt.map(|d| d as u64))
            .map_err(|e| RegistryError(e.to_string()))
    }
}

fn op_str(op: ChangeOp) -> &'static str {
    match op {
        ChangeOp::Upsert => "upsert",
        ChangeOp::Delete => "delete",
    }
}

/// Whether an authorization failure is expected to resolve on its own (so the
/// consumer should wait and retry) rather than being permanent (skip past it).
/// See [`Replicator::ingest`] for the reasoning behind each case.
fn is_transient(err: &AuthError) -> bool {
    match err {
        // Owner not yet claimed / key not yet published / parent row not yet
        // replicated / control-plane blip — all resolve on their own; retry.
        AuthError::Unowned | AuthError::UnknownNode | AuthError::Registry(_) => true,
        // Superseded by a rehoming, fenced, a bad signature, a non-home global
        // write, an unplaceable payload, or a row that doesn't match its domain
        // type — none of these ever becomes applicable.
        AuthError::NotOwner
        | AuthError::StaleEpoch
        | AuthError::WrongHome
        | AuthError::ScopeMismatch
        | AuthError::MalformedPayload
        // The signer isn't authorized by the community's founder-signed home
        // binding (a seizure attempt / superseded owner) — permanent; skip it.
        | AuthError::NotBoundHome
        | AuthError::Fed(_) => false,
    }
}

/// Whether an upsert's row document deserializes into the domain type its entity
/// expects. A verified event is authentic and owner-authorized, but that does not
/// make its bytes *well-formed*: a malicious (or buggy) owner could emit a row
/// whose `data` document is not a valid `User`/`Post`/… . Applying it would poison
/// the replica — every later strict-deserializing read of that row (and of any
/// list it appears in) would fail — so it is rejected at the boundary. Ballot
/// tables carry no `data` document and have nothing to validate here.
fn payload_is_well_formed(entity: &str, payload: &serde_json::Value) -> bool {
    use domain::{Comment, Demos, Membership, Post, Proposal, Report, Rule, Trial, User};
    // The domain document lives in the row's `data` column; a row without one
    // (the relational ballot tables) has no domain struct to validate.
    let Some(data) = payload.get("data") else {
        return true;
    };
    macro_rules! parses {
        ($t:ty) => {
            serde_json::from_value::<$t>(data.clone()).is_ok()
        };
    }
    match entity {
        "users" => parses!(User),
        "demoi" => parses!(Demos),
        "memberships" => parses!(Membership),
        "proposals" => parses!(Proposal),
        "rules" => parses!(Rule),
        "posts" => parses!(Post),
        "comments" => parses!(Comment),
        "reports" => parses!(Report),
        "trials" => parses!(Trial),
        _ => true,
    }
}

/// Applies a peer's change feed to the local replica, gated by full authorization.
pub struct Replicator {
    store: Arc<PostgresStore>,
    registry: Arc<dyn OwnershipRegistry>,
}

impl Replicator {
    pub fn new(store: Arc<PostgresStore>, registry: Arc<dyn OwnershipRegistry>) -> Self {
        Self { store, registry }
    }

    /// This node's replication cursor for `peer_node` — where a pull resumes.
    pub async fn cursor(&self, peer_node: i64) -> Result<i64> {
        self.store.replication_cursor(peer_node).await
    }

    /// Authorize events **in order** and apply the authorized *contiguous prefix*
    /// in a single batch.
    ///
    /// Each event must pass [`federation::authorize`] — authentic signature by the
    /// producer's registered key, that node being the community's current owner,
    /// and a non-stale epoch. This is the single choke point that closes review
    /// finding #1: an unsigned, forged, non-owner, or fenced event never reaches
    /// the store.
    ///
    /// A peer's feed is a **strictly ordered log**, so how a rejection is handled
    /// depends on whether it is *transient* or *permanent*:
    ///
    /// * **Transient** ([`is_transient`]) — the community's owner isn't known yet
    ///   (`Unowned`), the signer's key hasn't propagated (`UnknownNode`), or the
    ///   control plane blipped (`Registry`). Authorization **stops** here and the
    ///   cursor is left before the event, so a later pull retries it. This is what
    ///   lets a just-created community (claimed a few seconds after founding)
    ///   replicate once its owner claims it, rather than dropping those events.
    /// * **Permanent** — the event is signed by a node that is no longer the owner
    ///   (`NotOwner`) or under a fenced epoch (`StaleEpoch`), or its signature is
    ///   bad (`Fed`). Such an event is **superseded** (a rehoming happened) or
    ///   junk; it is **skipped** and the cursor advances past it. Leaving the
    ///   cursor stuck before it would stall *all* later replication from that peer
    ///   — exactly what happens to a returning old owner's feed after failover.
    ///
    /// The one guarantee that matters for durability holds throughout: a vote is
    /// only ever acknowledged after reaching a standby, so a permanently-skipped
    /// event is one that was never acked (or is a superseded duplicate).
    pub async fn ingest(&self, peer_node: i64, events: &[ChangeEvent]) -> Result<Ingested> {
        let mut changes: Vec<IncomingChange> = Vec::new();
        let mut rejected = Vec::new();
        // Highest seq we have deliberately handled (applied or permanently
        // skipped) before hitting a transient stop — the cursor may advance here.
        let mut handled_high = 0i64;
        let parents = StoreParents(self.store.clone());
        for ev in events {
            let seq = ev.peek().map(|p| p.seq as i64).unwrap_or(0);
            match authorize(self.registry.as_ref(), ev, &parents).await {
                Ok(part)
                    if part.op == ChangeOp::Upsert
                        && !payload_is_well_formed(&part.entity, &part.payload) =>
                {
                    // Authentic and owner-authorized, but the row document is
                    // malformed. Permanent: skip it and let the cursor step past,
                    // so one poisoned row can't stall the peer's whole feed.
                    handled_high = handled_high.max(part.seq as i64);
                    rejected.push(AuthError::MalformedPayload);
                }
                Ok(part) => {
                    handled_high = handled_high.max(part.seq as i64);
                    changes.push(IncomingChange {
                        seq: part.seq as i64,
                        entity: part.entity,
                        op: op_str(part.op).to_string(),
                        payload: part.payload,
                    });
                }
                Err(e) if is_transient(&e) => {
                    // Stop: leave the cursor before this event so it is retried.
                    rejected.push(e);
                    break;
                }
                Err(e) => {
                    // Permanent: skip it and let the cursor step past.
                    handled_high = handled_high.max(seq);
                    rejected.push(e);
                }
            }
        }
        let applied = self
            .store
            .apply_batch_advancing(peer_node, &changes, handled_high)
            .await?;
        Ok(Ingested { applied, rejected })
    }

    /// Apply events **pushed** to this node (the standby side of a synchronous
    /// vote), gated by the same authorization but WITHOUT advancing the
    /// replication cursor — see [`PostgresStore::apply_rows`]. This is a
    /// durability pre-apply; the ordered puller ([`ingest`](Self::ingest)) remains
    /// the cursor authority, so a push that lands ahead of the puller can't orphan
    /// the events in between. Authorized events are applied idempotently; a
    /// rejected one is simply not pre-applied (the puller will deliver it in order).
    pub async fn apply_pushed(&self, events: &[ChangeEvent]) -> Result<Ingested> {
        let mut changes: Vec<IncomingChange> = Vec::new();
        let mut rejected = Vec::new();
        let parents = StoreParents(self.store.clone());
        for ev in events {
            match authorize(self.registry.as_ref(), ev, &parents).await {
                Ok(part)
                    if part.op == ChangeOp::Upsert
                        && !payload_is_well_formed(&part.entity, &part.payload) =>
                {
                    rejected.push(AuthError::MalformedPayload);
                }
                Ok(part) => changes.push(IncomingChange {
                    seq: part.seq as i64,
                    entity: part.entity,
                    op: op_str(part.op).to_string(),
                    payload: part.payload,
                }),
                Err(e) => rejected.push(e),
            }
        }
        let applied = self.store.apply_rows(&changes).await?;
        Ok(Ingested { applied, rejected })
    }
}
