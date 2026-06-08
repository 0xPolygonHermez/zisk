// Job analytics panels: compact live signals first, 24h history below.

local p = import '../lib/panels.libsonnet';
local pg = import '../lib/postgres.libsonnet';
local t = import '../lib/thresholds.libsonnet';
local q = import '../lib/queries.libsonnet';
local grid = import '../lib/grid.libsonnet';
local c = import '../lib/colors.libsonnet';

[
  p.row(20, 'Reliability', grid.pos(0, 20, 24, 1), collapsed=false, panels=[]),

  p.stat(
    8, 'Proof Success Rate (24h)', grid.pos(0, 21, 4, 5),
    target=pg.success_rate_24h(limit=500),
    description='Fraction of terminal proof jobs that succeeded over the last 24 hours. Postgres history, restart-proof. Pair with live failure rate and failure reasons for reliability triage.',
    unit='percentunit',
    decimals=2,
    no_value='no matching terminal jobs in 24h',
    color_mode='value',
    text_mode='value',
    thresholds=[
      { color: c.critical, value: null },
      { color: c.warning, value: 0.95 },
      { color: c.healthy, value: 0.99 },
    ],
    value_size=36,
  ),

  p.bar_gauge(
    17, 'Proof Failures by Reason (selected range)', grid.pos(4, 21, 12, 5),
    targets=[pg.failure_reasons_selected_range(limit=500)],
    description='Restart-proof failure counts by reason over the dashboard time range, from Postgres proof history. Per-row failure messages remain in Recent Proof History.',
    unit='none',
    decimals=0,
    thresholds=[{ color: c.unknown, value: null }],
    title_size=13,
    value_size=22,
    min_viz_height=18,
    max_viz_height=32,
    values=true,
  ),

  p.timeseries(
    15, 'Proof Failure Rate by Kind (5m)', grid.pos(16, 21, 8, 5),
    targets=[p.prom_range(q.failure_rate_5m_by_kind, legend='{{kind}} failure rate')],
    description='Current coordinator process failure fraction over the last 5 minutes, split by workload kind. Live reliability signal, not historical count.',
    unit='percentunit', decimals=2,
    thresholds=t.failure_rate_pct,
  ),

  p.table(
    9, 'Proof Duration Stats (24h)', grid.pos(0, 26, 24, 4),
    targets=[
      pg.duration_stats_24h(limit=500),
    ],
    description='History-backed duration stats for terminal jobs in the last 24 hours. Values render as durations so normal proof time and failed-run time are directly comparable.',
    overrides=[
      p.by_name('Outcome', [p.prop_width(110)]),
      p.by_name('Jobs', [p.prop_unit('none'), p.prop_decimals(0)]),
      { matcher: { id: 'byRegexp', options: 'Avg|p50|p95|p99|Max' }, properties: [{ id: 'unit', value: 'ms' }, { id: 'decimals', value: 0 }] },
    ],
  ),
]
