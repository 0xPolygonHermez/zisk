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
