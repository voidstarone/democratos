-- Comment voting (the comment counterpart of post_votes). Feeds a comment's
-- display score and its author's per-community popularity metric.
--
-- Purely relational (no demos_id column), so — exactly like post_votes — the
-- capture function must derive the scoping community for correct per-community
-- replication: comment_vote → comment → post → demos.

CREATE TABLE IF NOT EXISTS comment_votes (
    comment_id BIGINT  NOT NULL,
    user_id    BIGINT  NOT NULL,
    up         BOOLEAN NOT NULL,
    PRIMARY KEY (comment_id, user_id)
);
CREATE INDEX IF NOT EXISTS comment_votes_user ON comment_votes (user_id);

-- Capture its row changes into the replication outbox, like every other table.
CREATE TRIGGER outbox_comment_votes
    AFTER INSERT OR UPDATE OR DELETE ON comment_votes
    FOR EACH ROW EXECUTE FUNCTION democratos_outbox();

-- Extend the capture function to scope comment_votes to their community.
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
        END IF;
    END IF;

    INSERT INTO outbox (entity, op, demos_id, row)
    VALUES (TG_TABLE_NAME, o, d, r);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
