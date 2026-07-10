//! The replicated-table allowlist and the payload redaction it drives.

use serde_json::Value;

/// A replicated table, how to identify one of its rows, the exact columns a peer
/// may populate, and any credential fields to strip. Every field is a compile-time
/// literal; the payload only supplies bound values (and only for these columns).
pub(crate) struct EntitySpec {
    pub(crate) table: &'static str,
    /// A WHERE predicate over `$1` (the row JSON) identifying the primary key.
    pub(crate) pk_predicate: &'static str,
    /// The columns an incoming upsert is allowed to write — an allowlist, so a peer
    /// can never set a column the entity's replication doesn't intend (e.g. one
    /// added by a later migration). The insert lists exactly these; any other key
    /// in the payload is ignored.
    pub(crate) columns: &'static [&'static str],
    /// Keys to drop from the row's `data` JSONB document before applying — for
    /// credential material a peer must never be able to set. The outbox already
    /// redacts these on the *sending* side; stripping again on ingest means a
    /// hand-crafted event from a malicious peer can't smuggle them back in.
    pub(crate) redact_data: &'static [&'static str],
}

/// The allowlist. Returns `None` for any entity this node does not replicate —
/// the guard that keeps an attacker from injecting an arbitrary table name.
pub(crate) fn entity_spec(entity: &str) -> Option<EntitySpec> {
    let spec = |table, pk_predicate, columns, redact_data| EntitySpec {
        table,
        pk_predicate,
        columns,
        redact_data,
    };
    Some(match entity {
        // `users` is global (not community-scoped), so credential material must
        // never be accepted from a peer — verification belongs on the home node.
        "users" => spec(
            "users",
            "id = ($1->>'id')::bigint",
            &["id", "handle", "created_at", "data"],
            &["password_hash", "email"],
        ),
        "demoi" => spec(
            "demoi",
            "id = ($1->>'id')::bigint",
            &["id", "slug", "created_at", "data"],
            &[],
        ),
        "proposals" => spec(
            "proposals",
            "id = ($1->>'id')::bigint",
            &["id", "demos_id", "data"],
            &[],
        ),
        "rules" => spec(
            "rules",
            "id = ($1->>'id')::bigint",
            &["id", "demos_id", "active", "data"],
            &[],
        ),
        "posts" => spec(
            "posts",
            "id = ($1->>'id')::bigint",
            &["id", "demos_id", "author", "created_at", "data"],
            &[],
        ),
        "comments" => spec(
            "comments",
            "id = ($1->>'id')::bigint",
            &["id", "post_id", "author", "created_at", "data"],
            &[],
        ),
        "reports" => spec(
            "reports",
            "id = ($1->>'id')::bigint",
            &["id", "demos_id", "is_open", "data"],
            &[],
        ),
        "trials" => spec(
            "trials",
            "id = ($1->>'id')::bigint",
            &["id", "demos_id", "verdict", "data"],
            &[],
        ),
        "memberships" => spec(
            "memberships",
            "user_id = ($1->>'user_id')::bigint AND demos_id = ($1->>'demos_id')::bigint",
            &["user_id", "demos_id", "tier", "enfranchised_at", "data"],
            &[],
        ),
        "votes" => spec(
            "votes",
            "proposal_id = ($1->>'proposal_id')::bigint AND voter_id = ($1->>'voter_id')::bigint",
            &["proposal_id", "voter_id", "aye", "weight"],
            &[],
        ),
        "post_votes" => spec(
            "post_votes",
            "post_id = ($1->>'post_id')::bigint AND user_id = ($1->>'user_id')::bigint",
            &["post_id", "user_id", "up"],
            &[],
        ),
        "comment_votes" => spec(
            "comment_votes",
            "comment_id = ($1->>'comment_id')::bigint AND user_id = ($1->>'user_id')::bigint",
            &["comment_id", "user_id", "up"],
            &[],
        ),
        "jury_ballots" => spec(
            "jury_ballots",
            "trial_id = ($1->>'trial_id')::bigint AND juror_id = ($1->>'juror_id')::bigint",
            &["trial_id", "juror_id", "guilty", "weight"],
            &[],
        ),
        _ => return None,
    })
}

/// Drop `redact` keys from a payload's `data` object, returning an owned value only
/// when something was removed (else the original is cloned untouched). Used to keep
/// credential fields out of an ingested `users` row.
pub(crate) fn redacted_payload(payload: &Value, redact: &[&str]) -> Value {
    let mut out = payload.clone();
    if !redact.is_empty() {
        if let Some(data) = out.get_mut("data").and_then(Value::as_object_mut) {
            for key in redact {
                data.remove(*key);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{entity_spec, redacted_payload};
    use serde_json::json;

    #[test]
    fn users_ingest_strips_credential_fields_from_data() {
        // A malicious peer hand-crafts a users upsert carrying credentials. On
        // ingest they must be dropped, so a peer can never set another node's
        // password hash or email.
        let spec = entity_spec("users").unwrap();
        assert_eq!(spec.redact_data, &["password_hash", "email"]);
        let payload = json!({
            "id": 1,
            "handle": "alice",
            "data": { "handle": "alice", "password_hash": "$argon2...", "email": "a@b.c", "public_key": "ab" }
        });
        let cleaned = redacted_payload(&payload, spec.redact_data);
        let data = cleaned.get("data").unwrap();
        assert!(data.get("password_hash").is_none(), "hash must be stripped");
        assert!(data.get("email").is_none(), "email must be stripped");
        // Non-credential fields survive.
        assert_eq!(data.get("public_key").unwrap(), "ab");
        assert_eq!(data.get("handle").unwrap(), "alice");
    }

    #[test]
    fn non_redacted_entities_pass_through_untouched() {
        let spec = entity_spec("proposals").unwrap();
        assert!(spec.redact_data.is_empty());
        let payload = json!({ "id": 1, "demos_id": 2, "data": { "status": "Open" } });
        assert_eq!(redacted_payload(&payload, spec.redact_data), payload);
    }

    #[test]
    fn every_entity_has_a_nonempty_column_allowlist() {
        for e in [
            "users", "demoi", "proposals", "rules", "posts", "comments", "reports",
            "trials", "memberships", "votes", "post_votes", "comment_votes", "jury_ballots",
        ] {
            assert!(!entity_spec(e).unwrap().columns.is_empty(), "{e} has columns");
        }
        assert!(entity_spec("outbox").is_none(), "outbox is not replicable");
    }
}
