CREATE TABLE IF NOT EXISTS job_history_jobs (
    job_id UUID PRIMARY KEY,
    coordinator_id TEXT NOT NULL DEFAULT 'default',
    hash_id TEXT NOT NULL,
    state TEXT NOT NULL,
    proof_type TEXT NOT NULL,
    received_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT,
    instances BIGINT,
    executed_steps BIGINT,
    agg_worker_id TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS job_history_job_workers (
    job_id UUID NOT NULL REFERENCES job_history_jobs(job_id) ON DELETE CASCADE,
    worker_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'participant',
    PRIMARY KEY (job_id, worker_id, role)
);

CREATE TABLE IF NOT EXISTS job_history_phase_events (
    id BIGSERIAL PRIMARY KEY,
    job_id UUID NOT NULL REFERENCES job_history_jobs(job_id) ON DELETE CASCADE,
    phase TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('started', 'ended')),
    occurred_at TIMESTAMPTZ NOT NULL,
    duration_ms BIGINT,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (job_id, phase, event_type, occurred_at)
);

CREATE INDEX IF NOT EXISTS idx_job_history_jobs_received_at
    ON job_history_jobs(received_at DESC NULLS LAST, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_job_history_jobs_coordinator_received_at
    ON job_history_jobs(coordinator_id, received_at DESC NULLS LAST, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_job_history_phase_events_job
    ON job_history_phase_events(job_id, phase, occurred_at);
