//! The `PostgresStore` replication operations: outbox draining, applying verified
//! peer changes, and the small bookkeeping the federation layer builds on.

use serde_json::Value;

use app::{Result, StoreError};

use super::apply_mode::{apply_one, ApplyMode};
use super::incoming_change::IncomingChange;
use super::outbox_record::OutboxRecord;
use crate::postgres_store::store_err;
use crate::PostgresStore;

/// Disk-exhaustion backstop for the outbox: rows older than this are force-pruned
/// regardless of any peer's cursor. Cursor-based pruning alone lets a single
/// stuck peer pin retention forever, so the outbox would grow without bound; this
/// bounds it by age at the cost of forcing a badly-lagging peer into a full
/// re-sync. Seven days is generous headroom over normal replication lag.
const MAX_OUTBOX_AGE_SECS: i64 = 7 * 24 * 60 * 60;

/// Wall-clock seconds since the Unix epoch, matching the outbox `at` column's
/// `extract(epoch from now())` default. Saturates to 0 before 1970.
fn now_epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl PostgresStore {
    /// Read up to `limit` outbox events after sequence `after_seq`, oldest first.
    /// The change feed serves these (after signing each).
    pub async fn outbox_since(&self, after_seq: i64, limit: i64) -> Result<Vec<OutboxRecord>> {
        let rows: Vec<(i64, String, String, Option<i64>, Value)> = sqlx::query_as(
            "SELECT seq, entity, op, demos_id, row FROM outbox
             WHERE seq > $1 ORDER BY seq LIMIT $2",
        )
        .bind(after_seq)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(rows
            .into_iter()
            .map(|(seq, entity, op, demos, payload)| OutboxRecord {
                seq,
                entity,
                op,
                demos,
                payload,
            })
            .collect())
    }

    /// Apply a batch of verified peer changes to the local replica — idempotently,
    /// in order, in a single transaction, and without re-emitting them to this
    /// node's own outbox. Returns how many were actually applied.
    ///
    /// The caller must have already **verified each event's signature and the
    /// producer's ownership+epoch** (see [`federation::authorize`]); this method
    /// enforces the remaining data-layer guarantees:
    ///
    /// * **Ordering / no-replay (review #2):** the per-peer cursor is read `FOR
    ///   UPDATE`, changes are applied strictly ascending by `seq`, and any
    ///   `seq <= cursor` is skipped — so a stale or replayed event can never
    ///   revert newer state.
    /// * **Injection safety:** table names come only from the [`entity_spec`]
    ///   allowlist; all values are bound.
    /// * **Atomicity:** the cursor advances in the same transaction as the writes,
    ///   so a crash cannot leave the cursor ahead of the data.
    /// * **No loop-back:** the transaction is marked replicating so the capture
    ///   trigger stays silent.
    ///
    /// Batching many events into one transaction (review #7) turns a per-event
    /// fsync into one per batch.
    pub async fn apply_batch(&self, peer_node: i64, changes: &[IncomingChange]) -> Result<u64> {
        self.apply_batch_advancing(peer_node, changes, 0).await
    }

    /// Like [`apply_batch`](Self::apply_batch), but also advances the cursor to at
    /// least `advance_to` even beyond the last *applied* sequence.
    ///
    /// The consumer uses this to step over **permanently** unauthorizable events
    /// (a dethroned owner's fenced events after a rehoming) without applying them:
    /// leaving the cursor stuck before such an event would stall *all* later
    /// replication from that peer. The caller must guarantee `advance_to` covers
    /// only events it has deliberately handled (applied or permanently skipped),
    /// never an event that could still become applicable (see the puller).
    pub async fn apply_batch_advancing(
        &self,
        peer_node: i64,
        changes: &[IncomingChange],
        advance_to: i64,
    ) -> Result<u64> {
        // Apply strictly ascending by seq regardless of the caller's ordering.
        let mut ordered: Vec<&IncomingChange> = changes.iter().collect();
        ordered.sort_by_key(|c| c.seq);

        let mut tx = self.pool.begin().await.map_err(store_err)?;
        sqlx::query("SET LOCAL democratos.replicating = 'on'")
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;

        // Ensure the cursor row exists, then lock it so concurrent appliers for
        // the same peer serialize and can't both advance past a skipped event.
        sqlx::query(
            "INSERT INTO replication_cursor (peer_node, last_seq) VALUES ($1, 0)
             ON CONFLICT (peer_node) DO NOTHING",
        )
        .bind(peer_node)
        .execute(&mut *tx)
        .await
        .map_err(store_err)?;
        let cursor: i64 = sqlx::query_scalar(
            "SELECT last_seq FROM replication_cursor WHERE peer_node = $1 FOR UPDATE",
        )
        .bind(peer_node)
        .fetch_one(&mut *tx)
        .await
        .map_err(store_err)?;

        let mut applied = 0u64;
        let mut high = cursor;
        for ch in ordered {
            if ch.seq <= cursor {
                continue; // stale / already applied — never revert newer state
            }
            apply_one(&mut tx, ch, ApplyMode::Overwrite).await?;
            applied += 1;
            high = high.max(ch.seq);
        }
        // Step over deliberately-skipped permanent rejects so they can't stall
        // the feed (never below the current cursor, since `high` starts there).
        high = high.max(advance_to);

        sqlx::query("UPDATE replication_cursor SET last_seq = $2 WHERE peer_node = $1")
            .bind(peer_node)
            .bind(high)
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;

        tx.commit().await.map_err(store_err)?;
        Ok(applied)
    }

    /// Apply verified changes idempotently **without touching any replication
    /// cursor** — the durability pre-apply used by synchronous vote replication.
    ///
    /// The ordered [`apply_batch`](Self::apply_batch) feed (the async puller) is
    /// the sole authority over the per-peer cursor; a sync vote push, by
    /// contrast, delivers just *one vote's* events out of band so the vote is on
    /// two nodes before it is acked. If that push advanced the shared cursor it
    /// would leap over events the puller hasn't delivered yet, orphaning them.
    /// So the push applies its rows (an idempotent `DELETE`+`INSERT` by primary
    /// key) and leaves the cursor alone; the puller later re-delivers the same
    /// rows in order — a harmless no-op — and advances the cursor normally.
    pub async fn apply_rows(&self, changes: &[IncomingChange]) -> Result<u64> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        sqlx::query("SET LOCAL democratos.replicating = 'on'")
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        let mut applied = 0u64;
        for ch in changes {
            apply_one(&mut tx, ch, ApplyMode::InsertIfAbsent).await?;
            applied += 1;
        }
        tx.commit().await.map_err(store_err)?;
        Ok(applied)
    }

    /// Apply a single verified change (convenience over [`apply_batch`]).
    pub async fn apply_change(
        &self,
        peer_node: i64,
        seq: i64,
        entity: &str,
        op: &str,
        payload: &Value,
    ) -> Result<()> {
        self.apply_batch(
            peer_node,
            &[IncomingChange {
                seq,
                entity: entity.to_string(),
                op: op.to_string(),
                payload: payload.clone(),
            }],
        )
        .await
        .map(|_| ())
    }

    /// Drop outbox events that **every** peer has already acknowledged (review #6),
    /// bounding the outbox's growth. Deletes nothing while any peer is behind, and
    /// nothing when there are no peers (so a lone node never discards unshipped
    /// events). Returns how many rows were pruned.
    pub async fn prune_outbox(&self) -> Result<u64> {
        // Cursor-based prune: drop only events *every* peer has acknowledged.
        let acked = sqlx::query(
            "DELETE FROM outbox WHERE seq <= (SELECT MIN(last_seq) FROM replication_cursor)",
        )
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();

        // Age backstop: one stuck peer pins `MIN(last_seq)` forever, so the prune
        // above can stall while the outbox grows without bound. Force-drop rows
        // older than `MAX_OUTBOX_AGE_SECS` regardless of cursors, as a
        // disk-exhaustion safeguard. This can discard events a lagging peer never
        // consumed — that peer must then recover via a full re-sync — so first
        // count how many such rows we are about to force out, and log it.
        let cutoff = now_epoch_secs() - MAX_OUTBOX_AGE_SECS;
        let unconsumed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM outbox
             WHERE at < $1
               AND seq > COALESCE((SELECT MIN(last_seq) FROM replication_cursor), 0)",
        )
        .bind(cutoff)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;

        let aged = sqlx::query("DELETE FROM outbox WHERE at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(store_err)?
            .rows_affected();

        if unconsumed > 0 {
            eprintln!(
                "prune_outbox: age backstop force-deleted {unconsumed} outbox row(s) \
                 older than {MAX_OUTBOX_AGE_SECS}s that a lagging peer had not yet \
                 consumed; that peer must full re-sync"
            );
        }
        Ok(acked + aged)
    }

    /// Durably record a forwarded-command nonce. Returns `true` if `(node, nonce)`
    /// was newly recorded (admit the command) or `false` if it was already present
    /// (a replay — refuse). Opportunistically prunes entries past their expiry so
    /// the table stays bounded. Backs the durable [replay guard] so a captured
    /// command can't be replayed against a freshly-restarted owner.
    pub async fn remember_command_nonce(
        &self,
        node: i64,
        nonce: &str,
        now: i64,
        expires_at: i64,
    ) -> Result<bool> {
        // Prune anything too old to still be replayable (keeps the table small).
        sqlx::query("DELETE FROM command_nonces WHERE expires_at <= $1")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        let inserted = sqlx::query(
            "INSERT INTO command_nonces (node, nonce, expires_at) VALUES ($1, $2, $3) \
             ON CONFLICT DO NOTHING",
        )
        .bind(node)
        .bind(nonce)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(store_err)?
        .rows_affected();
        Ok(inserted == 1)
    }

    /// Atomically count one attempt against a fixed-window rate `bucket` and report
    /// whether it is within `max_per_window`. The whole read-reset-increment-decide
    /// is a single upsert, so it is correct under concurrency and shared across every
    /// replica pointed at this database. Returns `true` to admit, `false` if over cap.
    pub async fn admit_rate(
        &self,
        bucket: &str,
        max_per_window: i32,
        window_secs: i64,
        now: i64,
    ) -> Result<bool> {
        // On conflict: reset the window (start=now, count=1) if the current one has
        // elapsed, else increment. `rate_counters.*` reads the pre-update row.
        let count: i32 = sqlx::query_scalar(
            "INSERT INTO rate_counters (bucket, window_start, count) VALUES ($1, $2, 1) \
             ON CONFLICT (bucket) DO UPDATE SET \
                window_start = CASE WHEN $2 - rate_counters.window_start >= $3 \
                                    THEN $2 ELSE rate_counters.window_start END, \
                count = CASE WHEN $2 - rate_counters.window_start >= $3 \
                             THEN 1 ELSE rate_counters.count + 1 END \
             RETURNING count",
        )
        .bind(bucket)
        .bind(now)
        .bind(window_secs)
        .fetch_one(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(count <= max_per_window)
    }

    /// This community's secret signing seed (hex), if this node holds it. Only the
    /// home node ever has it — it is never replicated.
    pub async fn community_seed(&self, demos: i64) -> Result<Option<String>> {
        let seed: Option<String> =
            sqlx::query_scalar("SELECT seed FROM community_keys WHERE demos = $1")
                .bind(demos)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(seed)
    }

    /// Persist a community's signing seed (write-once — idempotent on repeat). The
    /// home node calls this the first time it mints a community's identity so the
    /// key survives restarts and can re-sign the home binding.
    pub async fn set_community_seed(&self, demos: i64, seed_hex: &str) -> Result<()> {
        sqlx::query("INSERT INTO community_keys (demos, seed) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(demos)
            .bind(seed_hex)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(())
    }

    /// The current highest outbox sequence (0 if empty). Capture it before a
    /// write to then drain exactly that write's events with [`outbox_since`].
    pub async fn outbox_head(&self) -> Result<i64> {
        let seq: Option<i64> = sqlx::query_scalar("SELECT MAX(seq) FROM outbox")
            .fetch_one(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(seq.unwrap_or(0))
    }

    /// The community a ballot's parent row belongs to, read from the local
    /// replica. Used by federation authorization to bind a `votes` / `post_votes`
    /// / `jury_ballots` event to the community of its `proposals` / `posts` /
    /// `trials` parent — a ballot carries no `demos_id` of its own.
    ///
    /// `table` is one of a fixed set of parent tables (never attacker-controlled);
    /// an unknown table is refused rather than interpolated into SQL.
    pub async fn parent_demos(&self, table: &str, id: i64) -> Result<Option<i64>> {
        let sql = match table {
            "proposals" => "SELECT demos_id FROM proposals WHERE id = $1",
            "posts" => "SELECT demos_id FROM posts WHERE id = $1",
            "trials" => "SELECT demos_id FROM trials WHERE id = $1",
            other => {
                return Err(StoreError::Store(format!(
                    "parent_demos: unknown table '{other}'"
                )))
            }
        };
        let demos: Option<i64> = sqlx::query_scalar(sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(demos)
    }

    /// The highest sequence this node has applied from `peer_node` (0 if none).
    pub async fn replication_cursor(&self, peer_node: i64) -> Result<i64> {
        let seq: Option<i64> =
            sqlx::query_scalar("SELECT last_seq FROM replication_cursor WHERE peer_node = $1")
                .bind(peer_node)
                .fetch_optional(&self.pool)
                .await
                .map_err(store_err)?;
        Ok(seq.unwrap_or(0))
    }
}
