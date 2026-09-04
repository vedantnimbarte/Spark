-- Core schema. Greenfield, so the full model lands in one migration rather than
-- a trail of near-empty ones.

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT        NOT NULL UNIQUE,
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Only the hash of the session token is stored, so a database leak does not
-- hand over live sessions.
CREATE TABLE sessions (
    token_hash TEXT PRIMARY KEY,
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sessions_user_id_idx ON sessions (user_id);

CREATE TABLE applications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id        UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Becomes a DNS label in the ingress host and the k8s namespace, so it is
    -- constrained to what both accept.
    name            TEXT        NOT NULL CHECK (name ~ '^[a-z0-9]([-a-z0-9]*[a-z0-9])?$' AND length(name) <= 40),
    git_repo        TEXT        NOT NULL,
    git_branch      TEXT        NOT NULL DEFAULT 'main',
    build_type      TEXT        NOT NULL DEFAULT 'dockerfile',
    dockerfile_path TEXT        NOT NULL DEFAULT 'Dockerfile',
    container_port  INT         NOT NULL DEFAULT 8080,
    cpu_limit       TEXT        NOT NULL DEFAULT '500m',
    memory_limit    TEXT        NOT NULL DEFAULT '512Mi',
    webhook_secret  TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, name)
);

CREATE TABLE deployments (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    app_id      UUID        NOT NULL REFERENCES applications (id) ON DELETE CASCADE,
    commit_sha  TEXT        NOT NULL,
    status      TEXT        NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'building', 'deploying', 'deployed', 'failed')),
    logs        TEXT        NOT NULL DEFAULT '',
    image_ref   TEXT,
    started_at  TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX deployments_app_created_idx ON deployments (app_id, created_at DESC);

CREATE TABLE domains (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    app_id      UUID        NOT NULL REFERENCES applications (id) ON DELETE CASCADE,
    domain_name TEXT        NOT NULL UNIQUE,
    -- Always 'none' in v1: TLS is deferred. Column exists so enabling
    -- cert-manager later is additive.
    ssl_status  TEXT        NOT NULL DEFAULT 'none'
                CHECK (ssl_status IN ('none', 'pending', 'ready', 'failed')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX domains_app_idx ON domains (app_id);

-- Key names only. Values live solely in the per-app Kubernetes Secret.
CREATE TABLE app_env_keys (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    app_id     UUID        NOT NULL REFERENCES applications (id) ON DELETE CASCADE,
    key        TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (app_id, key)
);

-- Durable work queue. Claimed with FOR UPDATE SKIP LOCKED; a job outlives
-- cluster downtime and is retried.
CREATE TABLE jobs (
    id         BIGSERIAL PRIMARY KEY,
    kind       TEXT        NOT NULL,
    payload    JSONB       NOT NULL,
    run_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    attempts   INT         NOT NULL DEFAULT 0,
    locked_at  TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX jobs_claim_idx ON jobs (run_at) WHERE locked_at IS NULL;
