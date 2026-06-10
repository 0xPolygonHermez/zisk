// Recent proof progress lanes backed by persisted phase history.

local p = import '../lib/panels.libsonnet';
local pg = import '../lib/postgres.libsonnet';
local tr = import '../lib/transforms.libsonnet';
local c = import '../lib/colors.libsonnet';
local grid = import '../lib/grid.libsonnet';

[
  p.bar_chart(
    16, 'Proof Phase Progress (latest jobs)', grid.pos(0, 9, 24, 9),
    targets=[pg.phase_progress_latest_jobs(limit=5)],
    description='One progress lane per recent proof. The lane label carries total duration; hover for per-phase durations.',
    x_field='Proof',
    orientation='horizontal',
    stacking='normal',
    unit='ms',
    decimals=1,
    show_value='never',
    x_tick_label_spacing=100,
    gradient_mode='opacity',
    fill_opacity=92,
    line_width=1,
    bar_radius=0.08,
    group_width=0.58,
    bar_width=0.96,
    transformations=[
      tr.organize(
        index_by_name={ Proof: 0, 'Job ID': 1, Program: 2, State: 3, Started: 4, Completed: 5, Contribution: 6, Prove: 7, 'Aggregate/Wrap': 8, Execution: 9 },
        exclude_by_name={ 'Job ID': true, Program: true, State: true, Started: true, Completed: true },
      ),
    ],
    overrides=[
      p.by_name('Proof', [p.prop_unit('none')]),
      p.by_name_color('Contribution', '#24483D'),
      p.by_name_color('Prove', c.victorian_peak),
      p.by_name_color('Aggregate/Wrap', c.soft_green),
      p.by_name_color('Execution', c.light_green),
    ],
  ),
]
