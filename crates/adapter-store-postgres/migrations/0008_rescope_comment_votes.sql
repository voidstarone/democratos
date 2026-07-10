-- Restore per-community scoping for comment_votes in the capture function.
--
-- Migration 0005 taught democratos_outbox() to derive a comment_vote's scoping
-- community via comment_vote -> comment -> post -> demos. The later credential
-- redaction migration (0007) redefined democratos_outbox() with CREATE OR REPLACE
-- but dropped that comment_votes branch, so comment_vote events regressed to
-- emitting demos_id = NULL — i.e. authorized globally, which is cross-community
-- forgery. This migration redefines the function with the FULL 0007 body (keeping
-- the users credential redaction) AND the comment_votes derivation restored.

CREATE OR REPLACE FUNCTION democratos_outbox() RETURNS trigger AS $$
DECLARE
    r JSONB;
    o TEXT;
    d BIGINT;
BEGIN
    IF current_setting('democratos.replicating', true) = 'on' THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'DELETE' THEN
        r := to_jsonb(OLD);
        o := 'delete';
    ELSE
        r := to_jsonb(NEW);
        o := 'upsert';
    END IF;

    -- Never publish credentials: drop them from the replicated user document.
    IF TG_TABLE_NAME = 'users' AND r ? 'data' THEN
        r := jsonb_set(r, '{data}', (r->'data') - 'password_hash' - 'email');
    END IF;

    d := (r->>'demos_id')::BIGINT;

    IF d IS NULL THEN
        IF TG_TABLE_NAME = 'demoi' THEN
            d := (r->>'id')::BIGINT;             -- a community is scoped to itself
        ELSIF TG_TABLE_NAME = 'votes' THEN
            SELECT demos_id INTO d FROM proposals WHERE id = (r->>'proposal_id')::BIGINT;
        ELSIF TG_TABLE_NAME = 'post_votes' THEN
            SELECT demos_id INTO d FROM posts WHERE id = (r->>'post_id')::BIGINT;
        ELSIF TG_TABLE_NAME = 'comment_votes' THEN
            SELECT p.demos_id INTO d
            FROM comments c JOIN posts p ON p.id = c.post_id
            WHERE c.id = (r->>'comment_id')::BIGINT;
        ELSIF TG_TABLE_NAME = 'jury_ballots' THEN
            SELECT demos_id INTO d FROM trials WHERE id = (r->>'trial_id')::BIGINT;
        END IF;
    END IF;

    INSERT INTO outbox (entity, op, demos_id, row)
    VALUES (TG_TABLE_NAME, o, d, r);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
