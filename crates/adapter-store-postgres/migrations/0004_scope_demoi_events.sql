-- A community's own row (the `demoi` table) has no `demos_id` column, so its
-- create/update events were emitted with demos_id = NULL — i.e. authorized as a
-- *global* event (authenticity only), which would let any node with a published
-- key forge or mutate a community. A demos's own events must be owner-gated, so
-- scope them to the community's own id. (`users` stays genuinely global.)

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

    d := (r->>'demos_id')::BIGINT;

    IF d IS NULL THEN
        IF TG_TABLE_NAME = 'demoi' THEN
            d := (r->>'id')::BIGINT;             -- a community is scoped to itself
        ELSIF TG_TABLE_NAME = 'votes' THEN
            SELECT demos_id INTO d FROM proposals WHERE id = (r->>'proposal_id')::BIGINT;
        ELSIF TG_TABLE_NAME = 'post_votes' THEN
            SELECT demos_id INTO d FROM posts WHERE id = (r->>'post_id')::BIGINT;
        ELSIF TG_TABLE_NAME = 'jury_ballots' THEN
            SELECT demos_id INTO d FROM trials WHERE id = (r->>'trial_id')::BIGINT;
        END IF;
    END IF;

    INSERT INTO outbox (entity, op, demos_id, row)
    VALUES (TG_TABLE_NAME, o, d, r);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
