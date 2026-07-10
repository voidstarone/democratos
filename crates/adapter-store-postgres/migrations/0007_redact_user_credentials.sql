-- Keep credentials out of the change feed.
--
-- `users` is a *global* row, replicated to every node. The capture trigger
-- serialized the whole row with `to_jsonb(NEW)`, so a user's Argon2 password hash
-- and login email (both stored inside the `data` JSONB document) fanned out to
-- every peer and to anyone who could pull `/federation/changes`. Authentication
-- material must not ride the cross-node feed: a single node's compromise should
-- not hand an attacker every user's password hash and email fleet-wide.
--
-- Redefine the capture function to strip `password_hash` and `email` from a
-- `users` row's `data` document before it is written to the outbox. Every other
-- table is captured unchanged. The stripped fields are `Option`/`#[serde(default)]`
-- on the domain `User`, so a redacted row still deserializes on the receiver
-- (email/hash simply arrive as `None`). Consequence by design: a peer cannot serve
-- password login for a user it is not the home node of — credential verification
-- belongs on the account's home node, not on every replica.

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
        ELSIF TG_TABLE_NAME = 'jury_ballots' THEN
            SELECT demos_id INTO d FROM trials WHERE id = (r->>'trial_id')::BIGINT;
        END IF;
    END IF;

    INSERT INTO outbox (entity, op, demos_id, row)
    VALUES (TG_TABLE_NAME, o, d, r);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
