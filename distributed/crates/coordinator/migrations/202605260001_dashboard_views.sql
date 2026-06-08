CREATE OR REPLACE VIEW zisk_dashboard_job_summary AS
WITH phase_rollups AS (
    SELECT
        job_id,
        MAX(duration_ms) FILTER (
            WHERE phase = 'Contributions' AND event_type = 'ended'
        ) AS contributions_duration_ms,
        MAX(duration_ms) FILTER (
            WHERE phase = 'Prove' AND event_type = 'ended'
        ) AS prove_duration_ms,
        MAX(duration_ms) FILTER (
            WHERE phase = 'Aggregate' AND event_type = 'ended'
        ) AS aggregate_duration_ms,
        MAX(duration_ms) FILTER (
            WHERE phase = 'Execution' AND event_type = 'ended'
        ) AS execution_duration_ms
    FROM job_history_phase_events
    GROUP BY job_id
),
active_phases AS (
    SELECT DISTINCT ON (started.job_id)
        started.job_id,
        started.phase AS current_phase,
        started.occurred_at AS current_phase_started_at
    FROM job_history_phase_events started
    LEFT JOIN job_history_phase_events ended
        ON ended.job_id = started.job_id
        AND ended.phase = started.phase
        AND ended.event_type = 'ended'
        AND ended.occurred_at >= started.occurred_at
    WHERE started.event_type = 'started'
        AND ended.id IS NULL
    ORDER BY started.job_id, started.occurred_at DESC
),
worker_rollups AS (
    SELECT
        job_id,
        COUNT(DISTINCT worker_id) FILTER (WHERE role = 'participant') AS workers_count,
        STRING_AGG(DISTINCT worker_id, ', ' ORDER BY worker_id)
            FILTER (WHERE role = 'participant') AS workers
    FROM job_history_job_workers
    GROUP BY job_id
)
SELECT
    jobs.coordinator_id,
    jobs.job_id,
    SUBSTR(REPLACE(jobs.job_id::TEXT, '-', ''), 1, 8) AS job_label,
    COALESCE(NULLIF(jobs.program, ''), 'unknown') AS program,
    jobs.hash_id,
    jobs.state,
    CASE jobs.state
        WHEN 'Completed' THEN 'success'
        WHEN 'Failed' THEN 'failure'
        WHEN 'Cancelled' THEN 'cancelled'
        ELSE 'active'
    END AS outcome,
    jobs.failure_reason,
    jobs.proof_type,
    jobs.received_at,
    jobs.completed_at,
    jobs.duration_ms,
    COALESCE(worker_rollups.workers_count, 0) AS workers_count,
    COALESCE(worker_rollups.workers, '') AS workers,
    jobs.agg_worker_id,
    phase_rollups.contributions_duration_ms,
    phase_rollups.prove_duration_ms,
    phase_rollups.aggregate_duration_ms,
    phase_rollups.execution_duration_ms,
    CASE
        WHEN jobs.received_at IS NULL THEN NULL
        ELSE GREATEST(
            0,
            EXTRACT(EPOCH FROM (COALESCE(jobs.completed_at, NOW()) - jobs.received_at))::BIGINT
        )
    END AS age_seconds,
    CASE
        WHEN jobs.state LIKE 'Running%' THEN active_phases.current_phase
        ELSE NULL
    END AS current_phase,
    CASE
        WHEN jobs.state LIKE 'Running%' THEN active_phases.current_phase_started_at
        ELSE NULL
    END AS current_phase_started_at,
    CASE
        WHEN jobs.state LIKE 'Running%' AND active_phases.current_phase_started_at IS NOT NULL
        THEN GREATEST(
            0,
            EXTRACT(EPOCH FROM (NOW() - active_phases.current_phase_started_at))::BIGINT
        )
        ELSE NULL
    END AS current_phase_age_seconds,
    GREATEST(0, EXTRACT(EPOCH FROM (NOW() - jobs.updated_at))::BIGINT) AS last_update_age_seconds,
    jobs.instances,
    jobs.executed_steps,
    jobs.updated_at,
    COALESCE(jobs.completed_at, jobs.received_at, jobs.updated_at) AS sort_at
FROM job_history_jobs jobs
LEFT JOIN phase_rollups ON phase_rollups.job_id = jobs.job_id
LEFT JOIN active_phases ON active_phases.job_id = jobs.job_id
LEFT JOIN worker_rollups ON worker_rollups.job_id = jobs.job_id;

CREATE OR REPLACE VIEW zisk_dashboard_program_performance AS
SELECT
    program,
    MIN(hash_id) AS hash_id,
    COUNT(*) AS jobs_24h,
    COUNT(*) FILTER (WHERE outcome = 'success') AS success_24h,
    COUNT(*) FILTER (WHERE outcome = 'failure') AS failure_24h,
    COUNT(*) FILTER (WHERE outcome = 'cancelled') AS cancelled_24h,
    CASE
        WHEN COUNT(*) = 0 THEN NULL
        ELSE COUNT(*) FILTER (WHERE outcome = 'success')::DOUBLE PRECISION / COUNT(*)::DOUBLE PRECISION
    END AS success_rate,
    AVG(duration_ms) FILTER (WHERE duration_ms IS NOT NULL) AS avg_duration_ms,
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY duration_ms)
        FILTER (WHERE duration_ms IS NOT NULL) AS p50_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms)
        FILTER (WHERE duration_ms IS NOT NULL) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms)
        FILTER (WHERE duration_ms IS NOT NULL) AS p99_duration_ms,
    AVG(executed_steps::DOUBLE PRECISION / NULLIF(duration_ms::DOUBLE PRECISION / 1000.0, 0.0))
        FILTER (WHERE executed_steps IS NOT NULL AND duration_ms IS NOT NULL AND duration_ms > 0)
        AS avg_steps_per_second,
    MAX(sort_at) AS last_seen_at
FROM zisk_dashboard_job_summary
WHERE sort_at >= NOW() - INTERVAL '24 hours'
    AND outcome IN ('success', 'failure', 'cancelled')
GROUP BY program;

CREATE OR REPLACE VIEW zisk_dashboard_worker_errors AS
SELECT
    coordinator_id,
    worker_id,
    job_id,
    SUBSTR(REPLACE(job_id::TEXT, '-', ''), 1, 8) AS job_label,
    COALESCE(NULLIF(program, ''), 'unknown') AS program,
    hash_id,
    reason,
    message,
    occurred_at
FROM job_history_worker_errors;

CREATE INDEX IF NOT EXISTS idx_job_history_jobs_hash_received_at
    ON job_history_jobs(hash_id, received_at DESC NULLS LAST, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_job_history_jobs_state_received_at
    ON job_history_jobs(state, received_at DESC NULLS LAST, updated_at DESC);
