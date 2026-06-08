// Worker health, roster, and diagnostics panels.

local p = import '../lib/panels.libsonnet';
local q = import '../lib/queries.libsonnet';
local inf = import '../lib/infinity.libsonnet';
local pg = import '../lib/postgres.libsonnet';
local tr = import '../lib/transforms.libsonnet';
local c = import '../lib/colors.libsonnet';
local grid = import '../lib/grid.libsonnet';

[
  p.row(40, 'Worker Fleet', grid.pos(0, 121, 24, 1), collapsed=false, panels=[]),

  p.timeseries(
    42, 'Worker Heartbeat Lag by Worker', grid.pos(0, 122, 24, 8),
    targets=[p.prom_range(q.worker_heartbeat_lag_per_worker, legend='{{worker_id}}')],
    description='Seconds since each worker\'s last heartbeat reached the coordinator. Sourced from coordinator_worker_heartbeat_lag_seconds. Spikes above 30s mean lost heartbeats; above 90s the worker is treated as failed.',
    unit='s', decimals=1, min=0,
    thresholds=[
      { color: c.healthy, value: null },
      { color: c.warning, value: 30 },
      { color: c.critical, value: 90 },
    ],
    thresholds_style='area',
  ),

  p.table(
    44, 'Worker Roster', grid.pos(0, 130, 24, 6),
    targets=[
      inf.target(
        // No program/job_id filter: coord's workers endpoint drops idle
        // workers (program=null) when program filter is set, so the roster
        // would render empty whenever the fleet is between jobs. Fleet view
        // must always list every connected worker.
        '/api/v1/workers',
        columns=[
          inf.string('worker_id', 'Worker'),
          inf.string('status', 'Status'),
          inf.string('program', 'Program'),
          inf.string('job_label', 'Job'),
          inf.string('job_id', 'Job ID'),
          inf.string('phase', 'Phase'),
          inf.number('assigned_seconds', 'Assigned Age'),
          inf.timestamp('updated_at', 'Updated'),
        ],
      ),
    ],
    description='Live worker roster from /api/v1/workers. Coordinator-owned view of each connected worker, current assignment, and last-seen age.',
    transformations=[
      tr.organize(index_by_name={ Worker: 0, Status: 1, Program: 2, Job: 3, Phase: 4, 'Assigned Age': 5, Updated: 6, 'Job ID': 7 }),
    ],
    overrides=[
      p.by_name('Worker', [p.prop_width(190)]),
      p.by_name('Status', [p.prop_width(110)]),
      p.by_name('Program', [p.prop_width(180)]),
      p.by_name('Job', [p.prop_width(90)]),
      p.by_name('Job ID', [p.prop_width(285)]),
      p.by_name('Phase', [p.prop_width(120)]),
      p.by_name('Assigned Age', [p.prop_unit('s'), p.prop_decimals(0), p.prop_width(120)]),
      p.by_name('Updated', [p.prop_unit('dateTimeAsIso'), p.prop_width(160)]),
    ],
  ),

  // Deep-dive worker panels stay collapsed until an operator needs them.
  p.row(46, 'Worker Diagnostics', grid.pos(0, 136, 24, 1), collapsed=true, panels=[
    p.timeseries(
      47, 'Worker Assignments by Worker', grid.pos(0, 137, 12, 8),
      targets=[p.prom_range(q.worker_proof_assignments, legend='{{worker_id}} {{program}}')],
      description='Per-worker proof-job rate over time, derived from coordinator_worker_jobs_total. Use this to spot uneven load distribution across workers, idle workers, and program-level skew within the worker pool.',
      unit='ops', decimals=2,
      thresholds=[{ color: c.healthy, value: null }],
    ),

    p.table(
      45, 'Worker Error Events', grid.pos(12, 137, 12, 8),
      targets=[
        pg.worker_errors_recent(limit=50),
      ],
      description='Per-worker error events from job_history_worker_errors. Each row is one discrete failure with the raw error message and clickable Job ID.',
      no_value='no worker error events recorded',
      overrides=[
        p.by_name('When', [p.prop_unit('dateTimeAsIso'), p.prop_width(180)]),
        p.by_name('Worker', [p.prop_width(160)]),
        p.by_name('Program', [p.prop_width(130)]),
        p.by_name('Reason', [p.prop_width(140), p.prop_cell_color_bg, p.prop_mappings([{
          type: 'value',
          options: {
            heartbeat_lost: { color: c.critical, index: 0 },
            channel_closed: { color: c.critical, index: 1 },
            unreachable: { color: c.critical, index: 2 },
            setup_fail: { color: c.warning, index: 3 },
            prove_fail: { color: c.critical, index: 4 },
            agg_fail: { color: c.critical, index: 5 },
            unknown: { color: c.unknown, index: 6 },
          },
        }])]),
        p.by_name('Job ID', [p.prop_width(260)]),
      ],
    ),
  ]),
]
