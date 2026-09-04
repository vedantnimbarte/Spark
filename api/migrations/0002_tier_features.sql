-- Build logs move out of a TEXT column into append-only rows.
--
-- `deployments.logs` was appended to per line with `logs = logs || $1`, which
-- rewrites the whole row on every line under MVCC, and the log tail was read
-- with substring(), which detoasts the entire column on every poll. Both are
-- O(n) per line; together they are O(n^2) per build.
CREATE TABLE deployment_log_lines (
    deployment_id UUID   NOT NULL REFERENCES deployments (id) ON DELETE CASCADE,
    seq           BIGINT NOT NULL GENERATED ALWAYS AS IDENTITY,
    line          TEXT   NOT NULL,
    PRIMARY KEY (deployment_id, seq)
);

-- Preserve whatever the old column already holds.
INSERT INTO deployment_log_lines (deployment_id, line)
SELECT id, logs FROM deployments WHERE logs <> '';

ALTER TABLE deployments DROP COLUMN logs;

-- Rollback: a deployment that reuses an earlier deployment's image instead of
-- building. Recording the source keeps the history readable.
ALTER TABLE deployments
    ADD COLUMN rolled_back_from UUID REFERENCES deployments (id) ON DELETE SET NULL;

ALTER TABLE applications
    -- 0 is a valid setting: it stops an application without deleting it.
    ADD COLUMN replicas INT NOT NULL DEFAULT 1 CHECK (replicas BETWEEN 0 AND 10),
    -- Mirrors whether a git credential exists in the app's Kubernetes Secret,
    -- so the dashboard can show it without a cluster round-trip. The token
    -- itself is never stored here.
    ADD COLUMN git_credentials_set BOOLEAN NOT NULL DEFAULT false;

-- Any deploy still marked in flight belongs to a worker that is long gone;
-- the index below would reject them.
UPDATE deployments SET status = 'failed', finished_at = COALESCE(finished_at, now())
WHERE status IN ('pending', 'building', 'deploying');

-- Only one deploy may be in flight per application. Without this, two deploys
-- race and whichever finishes last wins, which may be the older commit.
CREATE UNIQUE INDEX deployments_one_active_per_app
    ON deployments (app_id)
    WHERE status IN ('pending', 'building', 'deploying');
