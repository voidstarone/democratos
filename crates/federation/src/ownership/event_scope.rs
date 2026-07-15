//! Where an event's authoritative community comes from, and the classifier that
//! derives it from the signed payload.

use domain::{origin_node, NodeId};

use crate::{ParentKind, SignedPart};

/// Where an event's **authoritative** community comes from — always derived from
/// the signed *payload*, never the envelope's self-declared `demos` (which a
/// malicious signer controls). This is the classification that lets
/// [`authorize`](crate::authorize) bind the ownership check to the row that is
/// actually being written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EventScope {
    /// A global row (a user account). Its sole authority is the node that minted
    /// the id (its high bits — see [`domain::origin_node`]); there is no per-
    /// community owner to consult.
    Global { home: NodeId },
    /// A demos-scoped row whose community is in the payload (`demos_id`, or, for
    /// the `demoi` community row itself, its own `id`).
    Demos(u64),
    /// A ballot whose community must be resolved from its parent row.
    ViaParent { kind: ParentKind, id: u64 },
    /// The payload lacked the key needed to place the row, or the entity is not
    /// one this node replicates. Refused outright.
    Indeterminate,
}

/// Read a `u64` id from a jsonb payload field. `to_jsonb(row)` renders a `bigint`
/// column as a JSON number, but tolerate a string encoding too so the classifier
/// never mis-scopes a legitimately-formed row.
fn payload_u64(payload: &serde_json::Value, key: &str) -> Option<u64> {
    match payload.get(key)? {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Classify an event by the community it authoritatively belongs to, reading only
/// its payload. The entity → scoping map mirrors the store's replication allowlist
/// and the outbox capture trigger; an entity outside it is [`EventScope::Indeterminate`].
pub fn event_scope(part: &SignedPart) -> EventScope {
    let id = |k| payload_u64(&part.payload, k);
    match part.entity.as_str() {
        // Global: authored by the account's home (minting) node.
        "users" => match id("id") {
            Some(uid) => EventScope::Global {
                home: origin_node(uid),
            },
            None => EventScope::Indeterminate,
        },
        // The community row itself is keyed by its own id.
        "demoi" => match id("id") {
            Some(d) => EventScope::Demos(d),
            None => EventScope::Indeterminate,
        },
        // Demos-scoped rows that carry their community directly.
        "memberships" | "proposals" | "rules" | "posts" | "reports" | "trials" => {
            match id("demos_id") {
                Some(d) => EventScope::Demos(d),
                None => EventScope::Indeterminate,
            }
        }
        // Comments carry no `demos_id` column of their own, so — like ballots — their
        // community is that of the post they hang on. Resolving via the parent post is
        // what lets comment events replicate at all AND keeps them owner-scoped: a
        // NULL-`demos_id` comment used to classify as `Indeterminate` (never
        // replicated), and never gained a cross-community authority.
        "comments" => match id("post_id") {
            Some(p) => EventScope::ViaParent {
                kind: ParentKind::Post,
                id: p,
            },
            None => EventScope::Indeterminate,
        },
        // Ballots inherit their parent's community.
        "votes" => match id("proposal_id") {
            Some(p) => EventScope::ViaParent {
                kind: ParentKind::Proposal,
                id: p,
            },
            None => EventScope::Indeterminate,
        },
        "post_votes" => match id("post_id") {
            Some(p) => EventScope::ViaParent {
                kind: ParentKind::Post,
                id: p,
            },
            None => EventScope::Indeterminate,
        },
        "jury_ballots" => match id("trial_id") {
            Some(t) => EventScope::ViaParent {
                kind: ParentKind::Trial,
                id: t,
            },
            None => EventScope::Indeterminate,
        },
        // A trial's gallery comment carries no demos_id; like a ballot, its
        // community is that of the trial it hangs on.
        "trial_comments" => match id("trial_id") {
            Some(t) => EventScope::ViaParent {
                kind: ParentKind::Trial,
                id: t,
            },
            None => EventScope::Indeterminate,
        },
        _ => EventScope::Indeterminate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChangeOp;
    use domain::compose_id;

    #[test]
    fn a_comment_scopes_to_its_parent_post_not_a_missing_demos_id() {
        // Regression (HIGH-1): a comment carries no demos_id of its own, so it must
        // resolve via its parent post — not classify as Indeterminate (which left
        // comment events un-replicable).
        let comment = SignedPart {
            node: 1,
            epoch: 1,
            seq: 1,
            demos: None,
            entity: "comments".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "id": 5, "post_id": 42, "author": 1, "body": "hi" }),
        };
        assert_eq!(
            event_scope(&comment),
            EventScope::ViaParent {
                kind: ParentKind::Post,
                id: 42
            }
        );
    }

    #[test]
    fn event_scope_is_derived_from_the_payload_not_the_envelope() {
        let scoped = SignedPart {
            node: 1,
            epoch: 1,
            seq: 1,
            demos: Some(7), // envelope value is ignored by event_scope
            entity: "posts".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "id": 1, "demos_id": 8 }),
        };
        assert_eq!(event_scope(&scoped), EventScope::Demos(8));

        let user = SignedPart {
            entity: "users".into(),
            payload: serde_json::json!({ "id": compose_id(NodeId(3), 9) }),
            ..scoped.clone()
        };
        assert_eq!(event_scope(&user), EventScope::Global { home: NodeId(3) });

        let ballot = SignedPart {
            entity: "post_votes".into(),
            payload: serde_json::json!({ "post_id": 42, "user_id": 1, "up": true }),
            ..scoped.clone()
        };
        assert_eq!(
            event_scope(&ballot),
            EventScope::ViaParent {
                kind: ParentKind::Post,
                id: 42
            }
        );

        let missing = SignedPart {
            entity: "memberships".into(),
            payload: serde_json::json!({ "user_id": 1 }),
            ..scoped
        };
        assert_eq!(event_scope(&missing), EventScope::Indeterminate);
    }
}
