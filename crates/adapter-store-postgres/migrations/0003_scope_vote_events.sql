-- Review #4: scope ballot events to their community.
--
-- votes / post_votes / jury_ballots are purely relational (no demos_id column),
-- so the original capture trigger emitted them with demos_id = NULL — meaning the
-- *most integrity-critical* events could not be filtered per community, and a node
-- replicating only community X would still receive X-and-everyone-else's ballots.
--
-- Redefine the capture function to derive the scoping community for those tables
-- (vote→proposal, post_vote→post, jury_ballot→trial). The triggers are unchanged;
-- they call this function by name, so replacing it is enough.

CREATE OR REPLACE FUNCTION democratos_outbox() RETURNS trigger AS $$
DECLARE
    r JSONB;
    o TEXT;
    d BIGINT;
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

    -- Most demos-scoped tables carry demos_id directly.
    d := (r->>'demos_id')::BIGINT;

    -- The relational ballot tables don't; look their community up by parent.
    IF d IS NULL THEN
        IF TG_TABLE_NAME = 'votes' THEN
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
