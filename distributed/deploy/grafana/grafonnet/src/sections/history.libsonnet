// Recent Proof History full-width table.

local p = import '../lib/panels.libsonnet';
local pg = import '../lib/postgres.libsonnet';
local c = import '../lib/colors.libsonnet';
local grid = import '../lib/grid.libsonnet';

local history_columns = [
  pg.string('coordinator_id', 'Coordinator'),
  pg.timestamp('received_at', 'Started'),
  pg.timestamp('completed_at', 'Completed'),
  pg.string('state', 'State'),
  pg.string('current_phase', 'Current Phase'),
  pg.number('age_seconds', 'Job Age'),
  pg.number('current_phase_age_seconds', 'Phase Age'),
  pg.number('last_update_age_seconds', 'Update Age'),
  pg.string('failure_reason', 'Failure Reason'),
  pg.timestamp('updated_at', 'Updated'),
  pg.number('duration_ms', 'Duration'),
  pg.number('contributions_duration_ms', 'Contribution'),
  pg.number('prove_duration_ms', 'Prove'),
  pg.number('aggregate_duration_ms', 'Aggregate'),
  pg.number('execution_duration_ms', 'Execution'),
  pg.number('executed_steps', 'Steps'),
  pg.number('workers_count', 'Workers'),
  pg.string('agg_worker_id', 'Aggregator'),
  pg.string('proof_type', 'Kind'),
  pg.string('program', 'Program'),
  pg.string('job_label', 'Job'),
  pg.string('job_id', 'Job ID'),
];

local state_mappings = [{
  type: 'value',
  options: {
    Completed: { text: 'Completed', color: c.outcome.success, index: 0 },
    Failed: { text: 'Failed', color: c.outcome.failure, index: 1 },
    Cancelled: { text: 'Cancelled', color: c.outcome.cancelled, index: 2 },
    Created: { text: 'Created', color: c.light_green, index: 3 },
    'Running (Contributions)': { text: 'Running: contribution', color: c.contribution, index: 4 },
    'Running (Prove)': { text: 'Running: prove', color: c.prove, index: 5 },
    'Running (Aggregate)': { text: 'Running: aggregate/wrap', color: c.aggregate, index: 6 },
    'Running (Execution)': { text: 'Running: execute', color: c.execute, index: 7 },
  },
}];

[
  p.table(
    10, 'Recent Proof History', grid.pos(0, 54, 24, 9),
    targets=[
      pg.recent_jobs(history_columns, limit=200),
    ],
    description='Postgres-backed recent proof history. Current panels above use /api/v1/jobs/current.',
    overrides=[
      p.by_name('Coordinator', [p.prop_width(140)]),
      p.by_name('Started', [p.prop_unit('dateTimeAsIso'), p.prop_width(160)]),
      p.by_name('Completed', [p.prop_unit('dateTimeAsIso'), p.prop_width(160)]),
      p.by_name('Updated', [p.prop_unit('dateTimeAsIso'), p.prop_width(160)]),
      p.by_name('State', [p.prop_mappings(state_mappings), p.prop_cell_color_text, p.prop_width(170)]),
      p.by_name('Current Phase', [p.prop_width(130)]),
      p.by_name('Job Age', [
        p.prop_unit('s'), p.prop_decimals(0),
        p.prop_width(95),
      ]),
      p.by_name('Phase Age', [
        p.prop_unit('s'), p.prop_decimals(0),
        p.prop_width(95),
      ]),
      p.by_name('Update Age', [
        p.prop_unit('s'), p.prop_decimals(0),
        p.prop_thresholds([
          { color: c.healthy, value: null },
          { color: c.warning, value: 60 },
          { color: c.critical, value: 300 },
        ]),
        p.prop_cell_color_bg, p.prop_width(100),
      ]),
      p.by_name('Duration', [p.prop_unit('ms'), p.prop_decimals(0), p.prop_width(100)]),
      p.by_name('Contribution', [p.prop_unit('ms'), p.prop_decimals(0), p.prop_width(110)]),
      p.by_name('Prove', [p.prop_unit('ms'), p.prop_decimals(0), p.prop_width(90)]),
      p.by_name('Aggregate', [p.prop_unit('ms'), p.prop_decimals(0), p.prop_width(100)]),
      p.by_name('Execution', [p.prop_unit('ms'), p.prop_decimals(0), p.prop_width(100)]),
      p.by_name('Steps', [p.prop_unit('short'), p.prop_decimals(2), p.prop_width(110)]),
      p.by_name('Workers', [p.prop_unit('none'), p.prop_decimals(0), p.prop_width(85)]),
      p.by_name('Aggregator', [p.prop_width(170)]),
      p.by_name('Failure Reason', [p.prop_width(260)]),
      p.by_name('Program', [p.prop_width(260)]),
      p.by_name('Job', [p.prop_width(90)]),
      p.by_name('Job ID', [p.prop_width(285)]),
    ],
  ),
]
