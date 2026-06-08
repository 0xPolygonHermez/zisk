CREATE TABLE IF NOT EXISTS job_event_log (
    event_id BIGSERIAL PRIMARY KEY,
    occurred_at TIMESTAMPTZ NOT NULL,
    coordinator_id TEXT NOT NULL,
    job_id UUID,
    worker_id TEXT,
    event_type TEXT NOT NULL,
    schema_version INT NOT NULL DEFAULT 1,
    dedupe_key TEXT NOT NULL,
    payload JSONB NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (dedupe_key)
);

CREATE INDEX IF NOT EXISTS idx_job_event_log_job
    ON job_event_log(job_id, event_id);

CREATE INDEX IF NOT EXISTS idx_job_event_log_worker
    ON job_event_log(worker_id, event_id)
    WHERE worker_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_job_event_log_occurred
    ON job_event_log(occurred_at DESC, event_id DESC);

CREATE INDEX IF NOT EXISTS idx_job_event_log_type_occurred
    ON job_event_log(event_type, occurred_at DESC);

CREATE TABLE IF NOT EXISTS projection_cursors (
    name TEXT PRIMARY KEY,
    last_event_id BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO projection_cursors (name, last_event_id)
VALUES ('job_history', 0)
ON CONFLICT (name) DO NOTHING;
