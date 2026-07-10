//! How a change is applied to the replica, and the routine that applies one.

use sqlx::{Postgres, Transaction};

use app::{Result, StoreError};

use super::entity_spec::{entity_spec, redacted_payload};
use super::incoming_change::IncomingChange;
use crate::postgres_store::store_err;

/// How a change is applied to the replica.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ApplyMode {
    /// `DELETE` then `INSERT` — full overwrite (upsert). Correct only when the
    /// caller serializes concurrent applies of the same row (the ordered puller
    /// does, via its `FOR UPDATE` cursor lock).
    Overwrite,
    /// `INSERT … ON CONFLICT DO NOTHING` — insert-if-absent, no delete. Safe
    /// under concurrency (two racers can't both duplicate-key), and idempotent.
    /// Used by the synchronous push, where many votes may be applied at once and
    /// a row already present (from another push or the puller) is simply skipped.
    InsertIfAbsent,
}

/// Apply one change to the replica within `tx`. Table names come only from the
/// [`entity_spec`] allowlist; the payload only supplies bound values.
pub(crate) async fn apply_one(
    tx: &mut Transaction<'_, Postgres>,
    ch: &IncomingChange,
    mode: ApplyMode,
) -> Result<()> {
    let spec = entity_spec(&ch.entity)
        .ok_or_else(|| StoreError::Store(format!("refusing unknown entity '{}'", ch.entity)))?;

    match ch.op.as_str() {
        "delete" => {
            let delete_sql = format!("DELETE FROM {} WHERE {}", spec.table, spec.pk_predicate);
            sqlx::query(&delete_sql)
                .bind(&ch.payload)
                .execute(&mut **tx)
                .await
                .map_err(store_err)?;
        }
        "upsert" => {
            // Strip any credential fields a peer must not set, then write *only* the
            // allowlisted columns. `cols` is a join of compile-time literals, so no
            // payload value ever reaches the SQL text — the row JSON supplies bound
            // values, and only for these columns.
            let payload = redacted_payload(&ch.payload, spec.redact_data);
            let cols = spec.columns.join(", ");
            let insert_sql = match mode {
                ApplyMode::Overwrite => {
                    // Delete-then-insert overwrites the prior version by primary key.
                    let delete_sql =
                        format!("DELETE FROM {} WHERE {}", spec.table, spec.pk_predicate);
                    sqlx::query(&delete_sql)
                        .bind(&ch.payload)
                        .execute(&mut **tx)
                        .await
                        .map_err(store_err)?;
                    // A plain INSERT here would abort the whole transaction — and thus
                    // permanently wedge the puller's cursor — if the incoming row
                    // collides on a *secondary* UNIQUE index (e.g. `users.handle`,
                    // `demoi.slug`) held by a different primary key that the pk-scoped
                    // DELETE above did not remove. `ON CONFLICT DO NOTHING` degrades
                    // such a collision into a skipped row rather than a stalled feed:
                    // the pk row is still overwritten in the normal case, and the
                    // pathological secondary-key clash is dropped (a permanent,
                    // single-row skip) instead of blocking all later replication.
                    format!(
                        "INSERT INTO {0} ({1}) SELECT {1} \
                         FROM jsonb_populate_record(NULL::{0}, $1) ON CONFLICT DO NOTHING",
                        spec.table, cols
                    )
                }
                // `ON CONFLICT DO NOTHING` with no target catches any unique/pk
                // violation and skips — concurrency-safe and idempotent.
                ApplyMode::InsertIfAbsent => format!(
                    "INSERT INTO {0} ({1}) SELECT {1} \
                     FROM jsonb_populate_record(NULL::{0}, $1) ON CONFLICT DO NOTHING",
                    spec.table, cols
                ),
            };
            sqlx::query(&insert_sql)
                .bind(&payload)
                .execute(&mut **tx)
                .await
                .map_err(store_err)?;
        }
        other => return Err(StoreError::Store(format!("unknown op '{other}'"))),
    }
    Ok(())
}
