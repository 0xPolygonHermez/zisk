// Current Proof live cards backed by /api/v1/jobs/current.

local p = import '../lib/panels.libsonnet';
local inf = import '../lib/infinity.libsonnet';
local c = import '../lib/colors.libsonnet';
local t = import '../lib/thresholds.libsonnet';
local grid = import '../lib/grid.libsonnet';

// Live cards intentionally stay unfiltered so short job labels do not break JSON endpoints.
local current_url = '/api/v1/jobs/current';
local neutral_threshold = [{ color: c.unknown, value: null }];
local worker_threshold = [{ color: c.unknown, value: null }, { color: c.healthy, value: 1 }];
local update_age_threshold = [
  { color: c.healthy, value: null },
  { color: c.warning, value: 60 },
  { color: c.critical, value: 300 },
];

local phase_mappings = [{
  type: 'value',
  options: {
    '0': { text: 'idle', color: c.unknown, index: 0 },
    '1': { text: 'queued', color: c.light_green, index: 1 },
    '2': { text: 'contribution', color: c.contribution, index: 2 },
    '3': { text: 'prove', color: c.prove, index: 3 },
    '4': { text: 'aggregate/wrap', color: c.aggregate, index: 4 },
    '5': { text: 'execute', color: c.execute, index: 5 },
    '255': { text: 'unknown', color: c.unknown, index: 6 },
  },
}];

[
  p.row(19, 'Current Proof', grid.pos(0, 4, 24, 1), collapsed=false, panels=[]),

  p.json_stat(
    26, 'Current Proof Phase (now)', grid.pos(0, 5, 6, 4),
    target=inf.target(current_url, columns=[inf.number('phase_code', 'Phase')]),
    description='Current phase for the active proof. Shows idle when the coordinator reports no active proof.',
    no_value='idle',
    mappings=phase_mappings,
    thresholds=neutral_threshold,
    color_mode='background',
  ),

  p.json_stat(
    22, 'Current Proof Duration (now)', grid.pos(6, 5, 4, 4),
    target=inf.target(current_url, columns=[inf.number('age_seconds', 'Duration')]),
    description='Elapsed time for the active proof. Terminal proof durations are shown in Proof Duration Stats and Recent Proof History.',
    unit='s',
    thresholds=t.observed_duration,
  ),

  p.json_stat(
    27, 'Current Phase Age (now)', grid.pos(10, 5, 4, 4),
    target=inf.target(current_url, columns=[inf.number('phase_age_seconds', 'Phase Age')]),
    description='Elapsed time inside the current phase. Expected phase length depends on program, workers, and GPU class.',
    unit='s',
    thresholds=t.observed_duration,
  ),

  p.json_stat(
    28, 'Progress Update Age (now)', grid.pos(14, 5, 4, 4),
    target=inf.target(current_url, columns=[inf.number('update_age_seconds', 'Update Age')]),
    description='Age of the latest progress snapshot for the active proof. Red means progress writes are stale while a run is still shown active.',
    unit='s',
    thresholds=update_age_threshold,
  ),

  p.json_stat(
    23, 'Workers Assigned (now)', grid.pos(18, 5, 6, 4),
    target=inf.target(current_url, columns=[inf.number('workers_count', 'Workers')]),
    description='Workers assigned to the active proof run. This is not a per-worker utilization metric.',
    thresholds=worker_threshold,
  ),
]
