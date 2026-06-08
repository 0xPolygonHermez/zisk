// Infrastructure panels stay collapsed by default.

local p = import '../lib/panels.libsonnet';
local q = import '../lib/queries.libsonnet';
local t = import '../lib/thresholds.libsonnet';
local c = import '../lib/colors.libsonnet';
local grid = import '../lib/grid.libsonnet';

[
  p.row(18, 'Infrastructure Health', grid.pos(0, 151, 24, 1), collapsed=true, panels=[
    p.timeseries(
      11, 'gRPC Request Rate by Status', grid.pos(0, 152, 8, 7),
      targets=[p.prom_range(q.grpc_request_rate, legend='{{method}} {{status}}')],
      description='Public API gRPC request rate by method and status. v0.18 emits status=ok/error, not outcome.',
      unit='reqps', decimals=2,
      thresholds=[{ color: c.healthy, value: null }],
      overrides=[
        { matcher: { id: 'byRegexp', options: '.* ok$' }, properties: [{ id: 'color', value: { mode: 'fixed', fixedColor: c.healthy } }] },
        { matcher: { id: 'byRegexp', options: '.* error$' }, properties: [
          { id: 'color', value: { mode: 'fixed', fixedColor: c.critical } },
          { id: 'custom.lineWidth', value: 2 },
        ] },
      ],
    ),

    p.timeseries(
      12, 'History Writer Queue and Drops', grid.pos(8, 152, 8, 7),
      targets=[
        p.prom_range(q.db_write_queue_depth, legend='queue depth', ref_id='A'),
        p.prom_range(q.db_write_dropped_range, legend='drops in range', ref_id='B'),
      ],
      description='Postgres history writer queue depth and drops in the selected time range. Write latency split out so this panel does not mix count and seconds.',
      unit='none', decimals=2,
      thresholds=[{ color: c.healthy, value: null }],
      overrides=[
        { matcher: { id: 'byName', options: 'queue depth' }, properties: [{ id: 'color', value: { mode: 'fixed', fixedColor: c.light_green } }] },
        { matcher: { id: 'byName', options: 'drops in range' }, properties: [{ id: 'color', value: { mode: 'fixed', fixedColor: c.critical } }] },
      ],
    ),

    p.timeseries(
      14, 'History DB Latency p95 by Operation', grid.pos(16, 152, 8, 7),
      targets=[p.prom_range(q.db_query_p95_by_op, legend='{{op}} p95')],
      description='p95 latency from the v0.18 summary metric coordinator_db_query_duration_seconds{quantile="0.95"}, split by history DB operation.',
      unit='s', decimals=3,
      thresholds=t.db_latency_s,
    ),
  ]),
]
