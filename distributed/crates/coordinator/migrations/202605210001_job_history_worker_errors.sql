CREATE TABLE IF NOT EXISTS job_history_worker_errors (
    id BIGSERIAL PRIMARY KEY,
    coordinator_id TEXT NOT NULL,
    worker_id TEXT NOT NULL,
    job_id UUID NOT NULL,
    hash_id TEXT NOT NULL,
    program TEXT NOT NULL,
    reason TEXT NOT NULL,
    message TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_worker_errors_occurred
    ON job_history_worker_errors (occurred_at DESC);

CREATE INDEX IF NOT EXISTS idx_worker_errors_worker
    ON job_history_worker_errors (worker_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS idx_worker_errors_job
    ON job_history_worker_errors (job_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS idx_worker_errors_program
    ON job_history_worker_errors (program, occurred_at DESC);
