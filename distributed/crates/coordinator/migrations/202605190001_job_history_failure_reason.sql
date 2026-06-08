ALTER TABLE job_history_jobs
    ADD COLUMN IF NOT EXISTS failure_reason TEXT;
