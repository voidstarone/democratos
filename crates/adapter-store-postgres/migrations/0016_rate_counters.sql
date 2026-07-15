-- Durable, shared fixed-window rate counters.
--
-- The delegated-account endpoints (a trusted issuer minting accounts, and verifying
-- a forwarded login) are rate-limited to bound abuse and password brute-forcing.
-- Holding that count only in process memory meant the cap was per-process: a
-- multi-replica issuer sharing one database multiplied the effective limit by the
-- replica count, and a restart reset it. Persisting the counter here makes the cap
-- hold across every replica and across restarts.
--
-- One row per bucket (e.g. `mint:<node>` or `auth:<handle>`); a fixed window that
-- resets when `now - window_start >= window_secs`. Rows are cheap and self-healing
-- (the window resets in place), so no pruning job is required.
CREATE TABLE IF NOT EXISTS rate_counters (
    bucket       TEXT   PRIMARY KEY,        -- '<kind>:<subject>' the cap applies to
    window_start BIGINT NOT NULL,           -- epoch secs the current window opened
    count        INT    NOT NULL            -- attempts counted in the current window
);
