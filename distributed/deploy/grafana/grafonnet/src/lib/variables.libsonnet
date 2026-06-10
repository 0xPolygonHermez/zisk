// Dashboard template variables. Keep variable names stable: downstream panels
// and runbooks refer to $program and $job_id.

local g = import 'github.com/grafana/grafonnet/gen/grafonnet-v11.2.0/main.libsonnet';
local q = import 'lib/queries.libsonnet';

[
  // No explicit allValue: Grafana drops `.*` into ${var:singlequote} unquoted
  // and breaks dashboard_filters_for. Default expands to the quoted member
  // list so the SQL parses; the ARRAY-overlap bypass in dashboard_filters_for
  // still catches the `$__all` sentinel when no values exist.
  g.dashboard.variable.query.new('coordinator', q.template_coordinator)
  + g.dashboard.variable.query.withDatasource('prometheus', 'prometheus')
  + g.dashboard.variable.query.generalOptions.withLabel('Coordinator')
  + g.dashboard.variable.query.selectionOptions.withMulti(true)
  + g.dashboard.variable.query.selectionOptions.withIncludeAll(true)
  + { current: { selected: false, text: 'All', value: '$__all' } },

  g.dashboard.variable.query.new('program', q.template_program)
  + g.dashboard.variable.query.withDatasource('prometheus', 'prometheus')
  + g.dashboard.variable.query.generalOptions.withLabel('Program')
  + g.dashboard.variable.query.selectionOptions.withMulti(true)
  + g.dashboard.variable.query.selectionOptions.withIncludeAll(true)
  + { current: { selected: false, text: 'All', value: '$__all' } },

  g.dashboard.variable.textbox.new('job_id', '')
  + g.dashboard.variable.textbox.generalOptions.withLabel('Job ID'),
]
