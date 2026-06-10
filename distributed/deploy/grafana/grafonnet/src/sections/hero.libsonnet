// Hero status cards for coordinator health, workload, and recent proof health.

local p = import '../lib/panels.libsonnet';
local q = import '../lib/queries.libsonnet';
local pg = import '../lib/postgres.libsonnet';
local c = import '../lib/colors.libsonnet';
local t = import '../lib/thresholds.libsonnet';
local grid = import '../lib/grid.libsonnet';

local coordinator_up_mappings = [{
  type: 'value',
  options: {
    '0': { text: 'offline', color: c.critical, index: 0 },
    '1': { text: 'online', color: c.healthy, index: 1 },
  },
}];

[
  p.stat(
    1, 'Coordinator Availability (now)', grid.pos(0, 0, 5, 4),
    target=p.prom_instant(q.cluster_up),
    description='Whether the selected coordinator process is visible to Prometheus. Worker attachment is shown separately so coordinator-down and no-worker states are not collapsed.',
    thresholds=t.up_down,
    color_mode='background',
    text_mode='auto',
    mappings=coordinator_up_mappings,
  ),

  p.stat(
    3, 'Workers Connected (now)', grid.pos(5, 0, 5, 4),
    target=p.prom_instant(q.connected_workers),
    description='Connected worker sessions reported by the v0.18 coordinator pool. This is pool availability, not per-worker utilization.',
    thresholds=t.worker_count,
  ),

  p.stat(
    2, 'Active Proofs (now)', grid.pos(10, 0, 4, 4),
    target=p.prom_instant(q.active_jobs),
    description='Current non-terminal proof jobs. Zero means idle, not success/failure.',
    thresholds=t.observed_count,
  ),

  p.stat(
    7, 'Average Proof Duration (last 10 successes)', grid.pos(14, 0, 5, 4),
    target=pg.avg_successful_proof_time_last_10(),
    description='Average total duration for the last 10 successful proof jobs. Failed and cancelled jobs stay split out in Proof Duration Stats.',
    unit='ms',
    decimals=1,
    thresholds=t.observed_duration,
    no_value='no successes',
  ),

  p.stat(
    6, 'Last Successful Proof Age (now)', grid.pos(19, 0, 5, 4),
    target=p.prom_instant(q.time_since_last_success),
    description='Seconds since the latest successful proof job. This gauge is seeded from Postgres history on coordinator startup and does not assume a fixed proof-time SLO.',
    unit='s',
    decimals=1,
    thresholds=t.observed_duration,
    no_value='none',
  ),
]
