-- Transactional outbox — the change feed that keeps peer replicas in sync.
--
-- Every authoritative write on this node is captured, in the *same transaction*
-- as the write itself, by an AFTER trigger. That transactional guarantee is why
-- a row and its outbox entry can never diverge: they commit together or not at
-- all. Capture is done in the database (via `to_jsonb(row)`), so the Rust store
-- methods need no change and cannot forget to emit an event.
--
-- A higher layer reads these rows, signs each as a `federation::ChangeEvent`, and
-- serves them on the change feed; peers verify the signature and apply them.

CREATE TABLE IF NOT EXISTS outbox (
    seq      BIGSERIAL PRIMARY KEY,           -- this node's total event order = peer cursor
    entity   TEXT   NOT NULL,                 -- table the row belongs to
    op       TEXT   NOT NULL,                 -- 'upsert' | 'delete'
    demos_id BIGINT,                          -- scoping community (NULL = global, e.g. users)
    row      JSONB  NOT NULL,                 -- to_jsonb(NEW) on upsert, to_jsonb(OLD) on delete
    at       BIGINT NOT NULL DEFAULT extract(epoch from now())::bigint
);
CREATE INDEX IF NOT EXISTS outbox_demos ON outbox (demos_id);

-- Per-peer replication cursor: the highest producer `seq` this node has applied
-- from each peer. Makes apply idempotent and resumable.
CREATE TABLE IF NOT EXISTS replication_cursor (
    peer_node BIGINT PRIMARY KEY,
    last_seq  BIGINT NOT NULL
);

-- The capture trigger. Two things make it safe:
--   * On a replica applying a peer's change, the apply path sets
--     `democratos.replicating = 'on'` for its transaction; the trigger sees that
--     and does NOT re-emit — so a replicated write never loops back into the feed.
--   * It only ever runs `to_jsonb` on the row, so it cannot be subverted by row
--     contents.
CREATE OR REPLACE FUNCTION democratos_outbox() RETURNS trigger AS $$
DECLARE
    r JSONB;
    o TEXT;
BEGIN
    IF current_setting('democratos.replicating', true) = 'on' THEN
        RETURN NULL; -- applying a replicated change: do not re-publish it
    END IF;
    IF TG_OP = 'DELETE' THEN
        r := to_jsonb(OLD);
        o := 'delete';
    ELSE
        r := to_jsonb(NEW);
        o := 'upsert';
    END IF;
    INSERT INTO outbox (entity, op, demos_id, row)
    VALUES (TG_TABLE_NAME, o, (r->>'demos_id')::BIGINT, r);
    RETURN NULL; -- AFTER trigger: return value is ignored
END;
$$ LANGUAGE plpgsql;

-- Attach to every replicated table (not to id_counters / outbox / cursor).
DO $$
DECLARE
    t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'users','demoi','memberships','proposals','votes','post_votes',
        'rules','posts','comments','reports','trials','jury_ballots'
    ] LOOP
        EXECUTE format('DROP TRIGGER IF EXISTS outbox_%1$s ON %1$s', t);
        EXECUTE format(
            'CREATE TRIGGER outbox_%1$s AFTER INSERT OR UPDATE OR DELETE ON %1$s
             FOR EACH ROW EXECUTE FUNCTION democratos_outbox()', t);
    END LOOP;
END;
$$;
