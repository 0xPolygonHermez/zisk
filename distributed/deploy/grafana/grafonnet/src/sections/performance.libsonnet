// Performance panels.

local p = import '../lib/panels.libsonnet';
local q = import '../lib/queries.libsonnet';
local pg = import '../lib/postgres.libsonnet';
local tr = import '../lib/transforms.libsonnet';
local c = import '../lib/colors.libsonnet';
local grid = import '../lib/grid.libsonnet';

[
  p.row(30, 'Proof Performance', grid.pos(0, 37, 24, 1), collapsed=false, panels=[]),

  p.histogram(
    31, 'Proof Duration Distribution (recent jobs)', grid.pos(0, 38, 12, 8),
    targets=[
      pg.recent_jobs([pg.number('duration_ms', 'Duration')], limit=500, where_extra='duration_ms IS NOT NULL'),
    ],
    description='Raw-value histogram from recent terminal job durations. Buckets honor dashboard filters and scale to the observed data range.',
    unit='ms',
    decimals=0,
    bucket_count=12,
    legend=false,
    axis_label='Total duration',
  ),

  p.bar_gauge(
    32, 'Proof Duration Quantiles (24h)', grid.pos(12, 38, 12, 8),
    targets=[
      pg.duration_quantiles_24h(limit=500),
    ],
    description='History-backed p50/p95/p99 total duration for terminal jobs in the last 24 hours.',
    unit='s', decimals=1,
    thresholds=[{ color: c.healthy, value: null }],
  ),

  p.xy_chart(
    36, 'Proof Duration by Cost (all proofs)', grid.pos(0, 46, 24, 8),
    targets=[pg.proof_duration_by_cost_all_proofs()],
    description='One dot per stored terminal proof. X is proving cost in million zkVM cycles, Y is total duration in minutes. Dashboard time range is not applied.',
    x_field='Cost (M cycles)',
    y_field='Duration (min)',
    x_axis_label='Cost (million cycles)',
    y_axis_label='Total duration (min)',
    x_unit='none', x_decimals=2,
    y_unit='none', y_decimals=2,
    point_size=7,
  ),

  p.row(35, 'Performance Trends', grid.pos(0, 70, 24, 1), collapsed=false, panels=[]),

  p.timeseries(
    134, 'Stage Utilization by Phase (15m)', grid.pos(0, 71, 24, 8),
    targets=[p.prom_range(q.phase_utilization_by_phase, legend='{{phase}}')],
    description='Rolling 15-minute phase workload from coordinator_phase_duration_seconds_sum. Values are proof-seconds per wall-clock second, so parallel workers can push the total above 1.',
    unit='none', decimals=2, min=0,
    fill_opacity=35,
    stacking_mode='normal',
    overrides=[
      p.by_name_color('Contributions', c.contribution),
      p.by_name_color('Prove', c.prove),
      p.by_name_color('Aggregate', c.aggregate),
      p.by_name_color('Execution', c.execute),
    ],
    legend_mode='list',
  ),

  p.timeseries(
    135, 'Executed Cycles Rate by Program (15m)', grid.pos(0, 79, 24, 8),
    targets=[p.prom_range(q.steps_per_second_by_program, legend='{{program}} cycles/s')],
    description='Rolling 15-minute per-program zkVM cycle throughput from coordinator_job_executed_steps_total. This shows actual proof work completed, not just elapsed time.',
    unit='ops', decimals=2, min=0,
    fill_opacity=15,
    legend_mode='list',
  ),

  p.timeseries(
    136, 'Proof Duration p95 by Program (15m)', grid.pos(0, 87, 24, 8),
    targets=[p.prom_range(q.duration_p95_by_program, legend='{{program}} p95')],
    description='Rolling 15-minute p95 in seconds by program from coordinator_job_duration_seconds_bucket. Uses increase() so sparse completed proofs still appear in the trend.',
    unit='s', decimals=2,
    thresholds=[{ color: c.healthy, value: null }],
    legend_mode='list',
  ),

  p.timeseries(
    137, 'Phase Duration p95 by Program and Phase (15m)', grid.pos(0, 95, 24, 8),
    targets=[p.prom_range(q.phase_duration_p95_by_program, legend='{{program}} {{phase}} p95')],
    description='Rolling 15-minute p95 in seconds by program and phase from coordinator_phase_duration_seconds_bucket. Uses increase() so sparse completed proofs still appear in the trend.',
    unit='s', decimals=2,
    thresholds=[{ color: c.healthy, value: null }],
    legend_mode='list',
  ),

  p.table(
    139, 'Program Performance Summary (24h)', grid.pos(0, 103, 24, 7),
    targets=[
      pg.program_performance_24h(limit=500),
    ],
    description='Postgres-backed per-program rollup. One row per program: job count, success rate, mean/p95/p99 duration, and average steps/s.',
    transformations=[
      tr.organize(index_by_name={ Program: 0, Jobs: 1, 'Success Rate': 2, Avg: 3, p95: 4, p99: 5, 'Steps/s': 6 }),
    ],
    overrides=[
      p.by_name('Program', [p.prop_width(260)]),
      p.by_name('Jobs', [p.prop_decimals(0), p.prop_width(100)]),
      p.by_name('Success Rate', [p.prop_unit('percentunit'), p.prop_decimals(2), p.prop_width(140)]),
      { matcher: { id: 'byRegexp', options: 'Avg|p95|p99' }, properties: [{ id: 'unit', value: 'ms' }, { id: 'decimals', value: 0 }] },
      p.by_name('Steps/s', [p.prop_unit('short'), p.prop_decimals(2), p.prop_width(140)]),
    ],
  ),
]
