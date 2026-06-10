// Coordinator availability and restart signals over the selected range.

local p = import '../lib/panels.libsonnet';
local q = import '../lib/queries.libsonnet';
local c = import '../lib/colors.libsonnet';

local up_down_mappings = [{
  type: 'value',
  options: {
    '0': { text: 'down', color: c.critical, index: 0 },
    '1': { text: 'up', color: c.healthy, index: 1 },
  },
}];

[
  p.row(50, 'Coordinator Runtime', { x: 0, y: 140, w: 24, h: 1 }, collapsed=false, panels=[]),

  p.state_timeline(
    51, 'Coordinator Availability Timeline', { x: 0, y: 141, w: 24, h: 5 },
    targets=[p.prom_range(q.coord_up_per_id, legend='{{coordinator_id}}')],
    description='Per-coordinator scrape state over the selected range. One lane per coordinator. Red segments = coord unreachable (in-flight jobs likely affected). Gaps reveal restarts and outages without reading numbers.',
    mappings=up_down_mappings,
    merge_values=true,
    show_value='never',
    row_height=0.9,
  ),

  p.timeseries(
    53, 'Coordinator Restarts (selected range)', { x: 0, y: 146, w: 24, h: 5 },
    targets=[p.prom_range(q.coordinator_restarts_range, legend='{{coordinator_id}} restarts')],
    description='Restart events per coordinator over the selected range. Spike = restart fired at that bucket. Derived from changes(coordinator_start_time_seconds) so the panel works without a dedicated restart counter.',
    unit='none', decimals=0,
    thresholds=[
      { color: c.healthy, value: null },
      { color: c.warning, value: 1 },
      { color: c.critical, value: 3 },
    ],
  ),
]
