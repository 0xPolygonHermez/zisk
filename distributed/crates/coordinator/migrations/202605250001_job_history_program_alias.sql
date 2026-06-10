ALTER TABLE job_history_jobs
    ADD COLUMN IF NOT EXISTS program TEXT;

UPDATE job_history_jobs
SET program = CASE
    WHEN hash_id = '' THEN 'unknown'
    ELSE SUBSTR(hash_id, 1, 8)
END
WHERE program IS NULL
    OR program = '';

ALTER TABLE job_history_jobs
    ALTER COLUMN program SET DEFAULT 'unknown',
    ALTER COLUMN program SET NOT NULL;
