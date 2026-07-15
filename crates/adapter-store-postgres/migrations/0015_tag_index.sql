-- Denormalized, pipe-wrapped tag index for fast tag search on posts and
-- communities.
--
-- Canonical tags live in each row's `data` JSONB document as a JSON array of
-- normalized `[a-z0-9-]` tags. Answering "which rows carry tag X" from that
-- array is awkward and hard to index, so we additionally materialize the tags
-- as one delimited string, `|tag1|tag2|`, in a generated `tags` column. A tag T
-- is then found with `tags LIKE '%|T|%'` — the surrounding bars make it an
-- exact-tag match, never a prefix collision (`|go|` never matches `|golang|`).
--
-- It is a STORED GENERATED column derived from `data`, which buys three things:
--   * it is recomputed automatically on every insert/update — no write-path code;
--   * existing rows are backfilled as the column is added by this migration;
--   * replicas stay correct for free — replication repopulates `data` and the
--     column recomputes. The column is deliberately absent from the replication
--     allowlist (see `entity_spec`), so a peer can never set it directly; it is
--     always derived locally from the authenticated `data`.

-- Join a row's `data->'tags'` array into the pipe-wrapped index string. Marked
-- IMMUTABLE (its output depends only on its argument) so it may drive a generated
-- column. Guards the non-array case explicitly rather than in a CASE branch,
-- since a set-returning call on a non-array would raise even in an untaken branch.
CREATE OR REPLACE FUNCTION tag_index(d jsonb) RETURNS text
    LANGUAGE plpgsql IMMUTABLE PARALLEL SAFE
AS $$
DECLARE
    joined text;
BEGIN
    IF jsonb_typeof(d -> 'tags') IS DISTINCT FROM 'array' THEN
        RETURN '';                       -- absent key / older datasets: no tags
    END IF;
    SELECT string_agg(elem, '|') INTO joined
    FROM jsonb_array_elements_text(d -> 'tags') AS elem;
    IF joined IS NULL THEN
        RETURN '';                       -- empty array
    END IF;
    RETURN '|' || joined || '|';
END;
$$;

ALTER TABLE posts ADD COLUMN tags TEXT NOT NULL GENERATED ALWAYS AS (tag_index(data)) STORED;
ALTER TABLE demoi ADD COLUMN tags TEXT NOT NULL GENERATED ALWAYS AS (tag_index(data)) STORED;

-- Founding petitions carry the founder's chosen tags until the community is born
-- (the founder is not present at the instant quorum founds it). This table is
-- column-based, not `data`-JSON, so the tags are a native `text[]` — no index
-- string, since petitions are not searched by tag.
ALTER TABLE foundings ADD COLUMN tags TEXT[] NOT NULL DEFAULT '{}';

-- A trigram GIN index makes the `LIKE '%|tag|%'` lookups indexed rather than a
-- sequential scan.
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS posts_tags_trgm ON posts USING gin (tags gin_trgm_ops);
CREATE INDEX IF NOT EXISTS demoi_tags_trgm ON demoi USING gin (tags gin_trgm_ops);
