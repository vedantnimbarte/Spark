-- Sessions are keyed by the token digest, which the dashboard must never see:
-- listing "your active sessions" needs a handle that is safe to send to the
-- browser and safe to accept back as the target of a revoke.
ALTER TABLE sessions
    ADD COLUMN id UUID NOT NULL DEFAULT gen_random_uuid();

CREATE UNIQUE INDEX sessions_id_idx ON sessions (id);
