// Grafana Postgres datasource target + SQL helpers for history/reporting panels.

local pg_ds = { type: 'grafana-postgresql-datasource', uid: 'zisk-postgres' };

{
  target(sql, ref_id='A', format='table'):: {
    refId: ref_id,
    datasource: pg_ds,
    format: format,
    rawQuery: true,
    rawSql: std.stripChars(sql, '\n '),
    editorMode: 'code',
  },

  field(expr, text):: std.format('%s AS "%s"', [expr, text]),
  string(expr, text):: self.field(expr, text),
  number(expr, text):: self.field(expr, text),
  timestamp(expr, text):: self.field(expr, text),

  // job_id filter matches full UUID, short job_label, or UUID prefix so the
  // operator can paste the 8-char label they see in tables without hitting
  // "no data". Coord-side endpoints still require the full UUID for live
  // panels (Infinity URLs intentionally omit job_id to keep working).
  dashboard_filters_for(program_col='program', coordinator_col='coordinator_id', job_id_col='job_id', job_label_col='job_label'):: std.format(|||
    AND (
      ARRAY[${coordinator:singlequote}]::text[] && ARRAY['', 'All', '$__all', '.*']::text[]
      OR %s = ANY(ARRAY[${coordinator:singlequote}]::text[])
    )
    AND (
      ARRAY[${program:singlequote}]::text[] && ARRAY['', 'All', '$__all', '.*']::text[]
      OR %s = ANY(ARRAY[${program:singlequote}]::text[])
    )
    AND (
      NULLIF(${job_id:singlequote}, '') IS NULL
      OR %s::text = ${job_id:singlequote}
      OR %s = ${job_id:singlequote}
      OR %s::text LIKE ${job_id:singlequote} || '%%'
    )
|||, [coordinator_col, program_col, job_id_col, job_label_col, job_id_col]),

  dashboard_filters:: self.dashboard_filters_for(),

  job_summary_cte:: |||
    WITH phase_summary AS (
      SELECT
        job_id,
        MAX(duration_ms) FILTER (WHERE phase = 'Contributions' AND event_type = 'ended') AS contributions_duration_ms,
        MAX(duration_ms) FILTER (WHERE phase = 'Prove' AND event_type = 'ended') AS prove_duration_ms,
        MAX(duration_ms) FILTER (WHERE phase = 'Aggregate' AND event_type = 'ended') AS aggregate_duration_ms,
        MAX(duration_ms) FILTER (WHERE phase = 'Execution' AND event_type = 'ended') AS execution_duration_ms
      FROM job_history_phase_events
      GROUP BY job_id
    ),
    open_phase AS (
      SELECT DISTINCT ON (started.job_id)
        started.job_id,
        started.phase AS current_phase,
        started.occurred_at AS current_phase_started_at
      FROM job_history_phase_events started
      WHERE started.event_type = 'started'
        AND NOT EXISTS (
          SELECT 1
          FROM job_history_phase_events ended
          WHERE ended.job_id = started.job_id
            AND ended.phase = started.phase
            AND ended.event_type = 'ended'
            AND ended.occurred_at >= started.occurred_at
        )
      ORDER BY started.job_id, started.occurred_at DESC
    ),
    worker_counts AS (
      SELECT
        job_id,
        COUNT(*) FILTER (WHERE role = 'participant') AS workers_count
      FROM job_history_job_workers
      GROUP BY job_id
    ),
    job_rows AS (
      SELECT
        s.coordinator_id,
        s.job_id::text AS job_id,
        s.job_label,
        s.program,
        s.hash_id,
        s.state,
        COALESCE(
          s.outcome,
          CASE s.state
            WHEN 'Completed' THEN 'success'
            WHEN 'Failed' THEN 'failure'
            WHEN 'Cancelled' THEN 'cancelled'
            ELSE NULL
          END
        ) AS outcome,
        s.received_at,
        s.completed_at,
        COALESCE(s.duration_ms, j.duration_ms) AS duration_ms,
        COALESCE(
          s.contributions_duration_ms,
          ps.contributions_duration_ms,
          CASE
            WHEN op.current_phase = 'Contributions'
              THEN GREATEST(0, EXTRACT(EPOCH FROM (NOW() - op.current_phase_started_at)) * 1000)::bigint
            ELSE NULL
          END
        ) AS contributions_duration_ms,
        COALESCE(
          s.prove_duration_ms,
          ps.prove_duration_ms,
          CASE
            WHEN op.current_phase = 'Prove'
              THEN GREATEST(0, EXTRACT(EPOCH FROM (NOW() - op.current_phase_started_at)) * 1000)::bigint
            ELSE NULL
          END
        ) AS prove_duration_ms,
        COALESCE(
          s.aggregate_duration_ms,
          ps.aggregate_duration_ms,
          CASE
            WHEN op.current_phase = 'Aggregate'
              THEN GREATEST(0, EXTRACT(EPOCH FROM (NOW() - op.current_phase_started_at)) * 1000)::bigint
            ELSE NULL
          END
        ) AS aggregate_duration_ms,
        COALESCE(
          ps.execution_duration_ms,
          CASE
            WHEN op.current_phase = 'Execution'
              THEN GREATEST(0, EXTRACT(EPOCH FROM (NOW() - op.current_phase_started_at)) * 1000)::bigint
            ELSE NULL
          END
        ) AS execution_duration_ms,
        j.executed_steps,
        COALESCE(s.workers_count, wc.workers_count, 0) AS workers_count,
        j.agg_worker_id,
        j.proof_type,
        s.failure_reason,
        CASE
          WHEN s.state LIKE 'Running%' THEN op.current_phase
          ELSE NULL
        END AS current_phase,
        CASE
          WHEN s.received_at IS NULL THEN NULL
          ELSE GREATEST(0, EXTRACT(EPOCH FROM (COALESCE(s.completed_at, NOW()) - s.received_at)))::bigint
        END AS age_seconds,
        CASE
          WHEN s.state LIKE 'Running%' AND op.current_phase_started_at IS NOT NULL
            THEN GREATEST(0, EXTRACT(EPOCH FROM (NOW() - op.current_phase_started_at)))::bigint
          ELSE NULL
        END AS current_phase_age_seconds,
        GREATEST(0, EXTRACT(EPOCH FROM (NOW() - j.updated_at)))::bigint AS last_update_age_seconds,
        j.updated_at,
        COALESCE(s.completed_at, s.received_at, j.updated_at) AS sort_at
      FROM zisk_dashboard_job_summary s
      JOIN job_history_jobs j ON j.job_id = s.job_id
      LEFT JOIN phase_summary ps ON ps.job_id = s.job_id
      LEFT JOIN open_phase op ON op.job_id = s.job_id
      LEFT JOIN worker_counts wc ON wc.job_id = s.job_id
    )
|||,

  recent_jobs(fields, limit=200, where_extra='TRUE', order_by='sort_at DESC NULLS LAST, job_id DESC', ref_id='A'):: self.target(
    std.format(|||
      %s
      SELECT
        %s
      FROM job_rows
      WHERE 1 = 1
      %s
        AND (%s)
      ORDER BY %s
      LIMIT %s
|||, [
      self.job_summary_cte,
      std.join(',\n  ', fields),
      self.dashboard_filters,
      where_extra,
      order_by,
      std.toString(limit),
    ]),
    ref_id=ref_id,
  ),

  duration_distribution(field, label, limit=500, buckets=12, ref_id='A'):: self.target(
    std.format(|||
      %s
      , sampled AS (
        SELECT (%s)::double precision / 1000.0 AS value
        FROM job_rows
        WHERE 1 = 1
      %s
        AND %s IS NOT NULL
        ORDER BY sort_at DESC NULLS LAST, job_id DESC
        LIMIT %s
      ),
      bounds AS (
        SELECT MIN(value) AS min_v, MAX(value) AS max_v
        FROM sampled
      ),
      binned AS (
        SELECT
          CASE
            WHEN b.max_v = b.min_v THEN 1
            ELSE LEAST(%s, GREATEST(1, width_bucket(s.value, b.min_v, b.max_v, %s)))
          END AS bucket,
          b.min_v,
          b.max_v
        FROM sampled s
        CROSS JOIN bounds b
        WHERE b.min_v IS NOT NULL
      )
      SELECT
        CASE
          WHEN max_v = min_v THEN ROUND(min_v::numeric, 1)::text || ' s'
          ELSE ROUND((min_v + (bucket - 1) * ((max_v - min_v) / %s))::numeric, 1)::text
            || '-' ||
            ROUND((min_v + bucket * ((max_v - min_v) / %s))::numeric, 1)::text || ' s'
        END AS "Range",
        COUNT(*)::bigint AS "%s"
      FROM binned
      GROUP BY bucket, min_v, max_v
      ORDER BY bucket
|||, [
      self.job_summary_cte,
      field,
      self.dashboard_filters,
      field,
      std.toString(limit),
      std.toString(buckets),
      std.toString(buckets),
      std.toString(buckets),
      std.toString(buckets),
      label,
    ]),
    ref_id=ref_id,
  ),

  // Single-row success rate over the last 24h of terminal jobs. Returned as
  // a fraction 0..1 so Grafana's `percentunit` formatter can render it.
  // Empty windows (no terminal jobs in last 24h) yield NULL; the consuming
  // stat panel renders its no_value placeholder rather than a bogus 0.
  success_rate_24h(limit=500, ref_id='A'):: self.target(
    std.format(|||
      %s
      , sampled AS (
        SELECT *
        FROM job_rows
        WHERE 1 = 1
      %s
        AND sort_at >= NOW() - INTERVAL '86400 seconds'
        ORDER BY sort_at DESC NULLS LAST, job_id DESC
        LIMIT %s
      ),
      terminal AS (
        SELECT outcome
        FROM sampled
        WHERE outcome IN ('success', 'failure', 'cancelled')
      )
      SELECT
        (COUNT(*) FILTER (WHERE outcome = 'success'))::double precision
          / NULLIF(COUNT(*), 0)::double precision AS "Success Rate"
      FROM terminal
|||, [self.job_summary_cte, self.dashboard_filters, std.toString(limit)]),
    ref_id=ref_id,
  ),

  failure_reasons_selected_range(limit=500, ref_id='A'):: self.target(
    std.format(|||
      %s
      , sampled AS (
        SELECT
          COALESCE(NULLIF(failure_reason, ''), 'unknown') AS reason
        FROM job_rows
        WHERE 1 = 1
      %s
        AND outcome IN ('failure', 'cancelled')
        AND $__timeFilter(sort_at)
        ORDER BY sort_at DESC NULLS LAST, job_id DESC
        LIMIT %s
      )
      SELECT
        reason AS "Reason",
        COUNT(*)::bigint AS "Failures"
      FROM sampled
      GROUP BY reason
      ORDER BY "Failures" DESC, "Reason" ASC
|||, [self.job_summary_cte, self.dashboard_filters, std.toString(limit)]),
    ref_id=ref_id,
  ),

  phase_progress_latest_jobs(limit=5, ref_id='A'):: self.target(
    std.format(|||
      %s
      , recent_jobs AS (
        SELECT
          job_id,
          job_label,
          program,
          state,
          outcome,
          received_at,
          completed_at,
          sort_at,
          (COALESCE(contributions_duration_ms, 0)
            + COALESCE(prove_duration_ms, 0)
            + COALESCE(aggregate_duration_ms, 0)
            + COALESCE(execution_duration_ms, 0))::double precision AS total_phase_ms,
          COALESCE(contributions_duration_ms, 0)::double precision AS contribution_ms,
          COALESCE(prove_duration_ms, 0)::double precision AS prove_ms,
          COALESCE(aggregate_duration_ms, 0)::double precision AS aggregate_ms,
          COALESCE(execution_duration_ms, 0)::double precision AS execution_ms,
          ROW_NUMBER() OVER (ORDER BY sort_at DESC NULLS LAST, job_id DESC) AS job_rank
        FROM job_rows
        WHERE 1 = 1
      %s
          AND (
            outcome IN ('success', 'failure', 'cancelled')
            OR state LIKE 'Running%%'
          )
          AND (COALESCE(contributions_duration_ms, 0)
            + COALESCE(prove_duration_ms, 0)
            + COALESCE(aggregate_duration_ms, 0)
            + COALESCE(execution_duration_ms, 0)) > 0
        ORDER BY sort_at DESC NULLS LAST, job_id DESC
        LIMIT %s
      )
      SELECT
        job_label || ' | total ' ||
        CASE
          WHEN total_phase_ms >= 3600000 THEN ROUND((total_phase_ms / 3600000.0)::numeric, 1)::text || 'h'
          WHEN total_phase_ms >= 60000 THEN ROUND((total_phase_ms / 60000.0)::numeric, 1)::text || 'm'
          ELSE ROUND((total_phase_ms / 1000.0)::numeric, 1)::text || 's'
        END AS "Proof",
        job_id AS "Job ID",
        program AS "Program",
        state AS "State",
        received_at AS "Started",
        completed_at AS "Completed",
        contribution_ms AS "Contribution",
        prove_ms AS "Prove",
        aggregate_ms AS "Aggregate/Wrap",
        execution_ms AS "Execution"
      FROM recent_jobs
      ORDER BY job_rank ASC
|||, [self.job_summary_cte, self.dashboard_filters, std.toString(limit)]),
    ref_id=ref_id,
  ),

  avg_successful_proof_time_last_10(limit=10, ref_id='A'):: self.target(
    std.format(|||
      %s
      , sampled AS (
        SELECT duration_ms
        FROM job_rows
        WHERE 1 = 1
      %s
        AND outcome = 'success'
        AND duration_ms IS NOT NULL
        ORDER BY sort_at DESC NULLS LAST, job_id DESC
        LIMIT %s
      )
      SELECT
        ROUND(AVG(duration_ms))::bigint AS "Avg Proof Time"
      FROM sampled
|||, [self.job_summary_cte, self.dashboard_filters, std.toString(limit)]),
    ref_id=ref_id,
  ),

  duration_stats_24h(limit=500, ref_id='A'):: self.target(
    std.format(|||
      %s
      , sampled AS (
        SELECT *
        FROM job_rows
        WHERE 1 = 1
      %s
        AND sort_at >= NOW() - INTERVAL '86400 seconds'
        ORDER BY sort_at DESC NULLS LAST, job_id DESC
        LIMIT %s
      ),
      terminal AS (
        SELECT outcome, duration_ms
        FROM sampled
        WHERE outcome IN ('success', 'failure', 'cancelled')
      )
      SELECT
        outcome AS "Outcome",
        COUNT(duration_ms)::bigint AS "Jobs",
        ROUND(AVG(duration_ms))::bigint AS "Avg",
        (percentile_disc(0.50) WITHIN GROUP (ORDER BY duration_ms) FILTER (WHERE duration_ms IS NOT NULL))::bigint AS "p50",
        (percentile_disc(0.95) WITHIN GROUP (ORDER BY duration_ms) FILTER (WHERE duration_ms IS NOT NULL))::bigint AS "p95",
        (percentile_disc(0.99) WITHIN GROUP (ORDER BY duration_ms) FILTER (WHERE duration_ms IS NOT NULL))::bigint AS "p99",
        MAX(duration_ms)::bigint AS "Max"
      FROM terminal
      GROUP BY outcome
      ORDER BY CASE outcome WHEN 'success' THEN 0 WHEN 'failure' THEN 1 WHEN 'cancelled' THEN 2 ELSE 3 END
|||, [self.job_summary_cte, self.dashboard_filters, std.toString(limit)]),
    ref_id=ref_id,
  ),

  duration_quantiles_24h(limit=500, ref_id='A'):: self.target(
    std.format(|||
      %s
      , sampled AS (
        SELECT *
        FROM job_rows
        WHERE 1 = 1
      %s
        AND sort_at >= NOW() - INTERVAL '86400 seconds'
        ORDER BY sort_at DESC NULLS LAST, job_id DESC
        LIMIT %s
      ),
      terminal AS (
        SELECT duration_ms::double precision / 1000.0 AS duration_seconds
        FROM sampled
        WHERE outcome IN ('success', 'failure', 'cancelled')
          AND duration_ms IS NOT NULL
      )
      SELECT
        percentile_disc(0.50) WITHIN GROUP (ORDER BY duration_seconds) AS "p50",
        percentile_disc(0.95) WITHIN GROUP (ORDER BY duration_seconds) AS "p95",
        percentile_disc(0.99) WITHIN GROUP (ORDER BY duration_seconds) AS "p99"
      FROM terminal
|||, [self.job_summary_cte, self.dashboard_filters, std.toString(limit)]),
    ref_id=ref_id,
  ),

  program_performance_24h(limit=500, ref_id='A'):: self.target(
    std.format(|||
      %s
      , sampled AS (
        SELECT *
        FROM job_rows
        WHERE 1 = 1
      %s
        AND sort_at >= NOW() - INTERVAL '86400 seconds'
        ORDER BY sort_at DESC NULLS LAST, job_id DESC
        LIMIT %s
      ),
      terminal AS (
        SELECT *
        FROM sampled
        WHERE outcome IN ('success', 'failure', 'cancelled')
      )
      SELECT
        program AS "Program",
        COUNT(*)::bigint AS "Jobs",
        (COUNT(*) FILTER (WHERE outcome = 'success'))::double precision / NULLIF(COUNT(*), 0)::double precision AS "Success Rate",
        ROUND(AVG(duration_ms))::bigint AS "Avg",
        (percentile_disc(0.95) WITHIN GROUP (ORDER BY duration_ms) FILTER (WHERE duration_ms IS NOT NULL))::bigint AS "p95",
        (percentile_disc(0.99) WITHIN GROUP (ORDER BY duration_ms) FILTER (WHERE duration_ms IS NOT NULL))::bigint AS "p99",
        AVG(
          CASE
            WHEN duration_ms > 0 AND executed_steps IS NOT NULL
              THEN executed_steps::double precision / (duration_ms::double precision / 1000.0)
            ELSE NULL
          END
        ) AS "Steps/s"
      FROM terminal
      GROUP BY program
      ORDER BY "Jobs" DESC, program ASC
|||, [self.job_summary_cte, self.dashboard_filters, std.toString(limit)]),
    ref_id=ref_id,
  ),

  // Failure-by-phase distribution for terminal Failed / Cancelled jobs over
  // the last 24h. The "phase at failure" is the last phase that has a
  // started event with no matching ended event for that job (mirrors the
  // open_phase CTE used by job_summary_cte). NULL phases (job died before
  // any phase started, e.g. setup hard-failure) are bucketed as `setup`.
  failure_phase_distribution_24h(limit=500, ref_id='A'):: self.target(
    std.format(|||
      %s
      , sampled AS (
        SELECT *
        FROM job_rows
        WHERE 1 = 1
      %s
        AND sort_at >= NOW() - INTERVAL '86400 seconds'
        ORDER BY sort_at DESC NULLS LAST, job_id DESC
        LIMIT %s
      ),
      terminal_failed AS (
        SELECT s.job_id, op.current_phase AS phase
        FROM sampled s
        LEFT JOIN open_phase op ON op.job_id::text = s.job_id
        WHERE s.outcome IN ('failure', 'cancelled')
      )
      SELECT
        COALESCE(phase, 'setup') AS "Phase",
        COUNT(*)::bigint AS "Failures"
      FROM terminal_failed
      GROUP BY COALESCE(phase, 'setup')
      ORDER BY "Failures" DESC, "Phase" ASC
|||, [self.job_summary_cte, self.dashboard_filters, std.toString(limit)]),
    ref_id=ref_id,
  ),

  proof_duration_by_cost_all_proofs(ref_id='A'):: self.target(
    std.format(|||
      %s
      , ordered AS (
        SELECT
          executed_steps::double precision / 1000000.0 AS cost_mcycles,
          duration_ms::double precision / 60000.0 AS duration_minutes,
          COALESCE(job_label, LEFT(job_id, 8)) || ' [' || COALESCE(program, '-') || ']' AS proof
        FROM job_rows
        WHERE outcome IN ('success', 'failure', 'cancelled')
          AND duration_ms IS NOT NULL
          AND executed_steps IS NOT NULL
        %s
      )
      SELECT
        cost_mcycles AS "Cost (M cycles)",
        duration_minutes AS "Duration (min)",
        proof AS "Proof"
      FROM ordered
      ORDER BY cost_mcycles ASC, duration_minutes ASC
|||, [self.job_summary_cte, self.dashboard_filters]),
    ref_id=ref_id,
  ),

  worker_errors_recent(limit=50, ref_id='A'):: self.target(
    std.format(|||
      SELECT
        occurred_at AS "When",
        worker_id AS "Worker",
        program AS "Program",
        reason AS "Reason",
        job_id::text AS "Job ID",
        COALESCE(message, '') AS "Message"
      FROM job_history_worker_errors
      WHERE 1 = 1
      %s
      ORDER BY occurred_at DESC, id DESC
      LIMIT %s
|||, [
      self.dashboard_filters_for(
        program_col='program',
        coordinator_col='coordinator_id',
        job_id_col='job_id::text',
        // Worker errors table has no job_label column; surface the leading
        // UUID slug as a synthetic label so the textbox short-prefix path
        // still matches against this table.
        job_label_col="LEFT(job_id::text, 8)",
      ),
      std.toString(limit),
    ]),
    ref_id=ref_id,
  ),
}
