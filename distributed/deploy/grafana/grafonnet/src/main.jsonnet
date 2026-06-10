// ZisK Coordinator dashboard grafonnet entrypoint.
//
// Build:   make build canonicalize         (default)
// Build:   jsonnet ... --tla-str env=prod  (multi-env when needed)

local g = import 'github.com/grafana/grafonnet/gen/grafonnet-v11.2.0/main.libsonnet';
local q = import 'lib/queries.libsonnet';
local variables = import 'lib/variables.libsonnet';

g.dashboard.new('ZisK Coordinator v0.18')
+ g.dashboard.withUid('zisk-dev')
+ g.dashboard.withRefresh('1s')
+ g.dashboard.withSchemaVersion(39)
+ g.dashboard.time.withFrom('now-2h')
+ g.dashboard.time.withTo('now')
+ g.dashboard.withTags(['zisk', 'coordinator', 'v0.18'])
+ g.dashboard.withTimezone('browser')
+ g.dashboard.withEditable(true)

+ g.dashboard.withVariables(variables)

+ g.dashboard.withAnnotations([
    g.dashboard.annotation.withName('Coordinator scrape state changed')
    + g.dashboard.annotation.withDatasource({ type: 'prometheus', uid: 'prometheus' })
    + g.dashboard.annotation.withExpr(q.annot_coord_scrape)
    + g.dashboard.annotation.withEnable(true),

    g.dashboard.annotation.withName('Coordinator process restarted')
    + g.dashboard.annotation.withDatasource({ type: 'prometheus', uid: 'prometheus' })
    + g.dashboard.annotation.withExpr(q.annot_coord_restart)
    + g.dashboard.annotation.withEnable(true),

    g.dashboard.annotation.withName('Worker pool changed')
    + g.dashboard.annotation.withDatasource({ type: 'prometheus', uid: 'prometheus' })
    + g.dashboard.annotation.withExpr(q.annot_worker_pool)
    + g.dashboard.annotation.withEnable(true),
  ])

+ g.dashboard.withPanels(
    (import 'sections/hero.libsonnet')
    + (import 'sections/proof_run_summary.libsonnet')
    + (import 'sections/lifecycle.libsonnet')
    + (import 'sections/job_details.libsonnet')
    + (import 'sections/performance.libsonnet')
    + (import 'sections/history.libsonnet')
    + (import 'sections/workers.libsonnet')
    + (import 'sections/coordinator_health.libsonnet')
    + (import 'sections/infrastructure.libsonnet'),
    setPanelIDs=false,
  )
