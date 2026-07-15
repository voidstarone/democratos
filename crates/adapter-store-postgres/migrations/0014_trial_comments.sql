-- Comments on trials — the community's public gallery discussion of a case.
-- Federated exactly like the trial they hang off (see 0002_outbox): a trial is
-- replicated across the community's nodes, so its discussion must be too. Purely
-- relational (no demos_id column), so — like jury_ballots — the capture function
-- derives the scoping community: trial_comment → trial → demos.

CREATE TABLE IF NOT EXISTS trial_comments (
    id       BIGINT PRIMARY KEY,
    trial_id BIGINT NOT NULL,
    data     JSONB  NOT NULL
);
-- A trial's discussion, read oldest-first by the caller.
CREATE INDEX IF NOT EXISTS trial_comments_trial ON trial_comments (trial_id, id);

-- Capture its row changes into the replication outbox, like every other
-- federated table.
CREATE TRIGGER outbox_trial_comments
    AFTER INSERT OR UPDATE OR DELETE ON trial_comments
    FOR EACH ROW EXECUTE FUNCTION democratos_outbox();

-- Extend the capture function to scope trial_comments to their community via the
-- trial they belong to (mirrors the jury_ballots branch).
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
            d := (r->>'id')::BIGINT;
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
        ELSIF TG_TABLE_NAME = 'trial_comments' THEN
            SELECT demos_id INTO d FROM trials WHERE id = (r->>'trial_id')::BIGINT;
        END IF;
    END IF;

    INSERT INTO outbox (entity, op, demos_id, row)
    VALUES (TG_TABLE_NAME, o, d, r);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
