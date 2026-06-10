# ZisK Coordinator Dashboard - grafonnet source

Grafonnet source for the ZisK Coordinator dashboard.

`src/**/*.jsonnet` and `src/**/*.libsonnet` are the reviewable source. The
provisioned `../dashboards/zisk-overview.json` file is treated as the generated
artifact until Grafana provisioning is switched to read the build output
directly.

## Toolchain

```
brew install jsonnet jsonnet-bundler
make deps
make build canonicalize validate diff
make sync
```

## Layout

```
src/main.jsonnet            entrypoint (TLA-parameterized for multi-env)
src/lib/
  colors.libsonnet          STAGE_COLORS palette
  queries.libsonnet         every PromQL string, named
  thresholds.libsonnet      named SLO threshold sets
  transforms.libsonnet      Grafana transformation helpers
  infinity.libsonnet        Infinity datasource target + column builders
  panels.libsonnet          panel constructors wrapping grafonnet API
  grid.libsonnet            grid position helper
src/sections/*.libsonnet    panel groups, one per visual row
```

Validator and smoke probe are Rust crates in the workspace:

```
distributed/crates/coordinator-contract/             extracts metric + route
                                                     contract from coord source
distributed/crates/coordinator-dashboard-validator/  static dashboard rules
distributed/crates/coordinator-dashboard-smoke/      live Grafana/Prom/coord probes
distributed/crates/coordinator-dashboard-integration/ observes a real proof run
                                                     through live JSON, Prom,
                                                     and Grafana Postgres panels
```

## Grafana version pinning

Grafonnet is pinned to `v11.2.0` in `jsonnetfile.json` to match the deployed
Grafana 11.2.0 (see `distributed/deploy/docker/compose.yaml`). Bump the pin
in lockstep when Grafana upgrades.

## Workflow

1. Edit `src/**/*.jsonnet` or `src/**/*.libsonnet`.
2. Run `make validate` - regenerates `known-contract.json` from coord source,
   builds the jsonnet, runs the Rust validator.
3. Run `make smoke` against a running Grafana/coordinator stack.
4. Run `make sync` to regenerate `../dashboards/zisk-overview.json`.

Do not hand-edit `../dashboards/zisk-overview.json` for dashboard behavior.
Change the Grafonnet source and sync the generated artifact.

The Current Proof section is card-based:

- `Current Proof Phase (now)`, `Current Proof Duration (now)`,
  `Current Phase Age (now)`, `Progress Update Age (now)`, and
  `Workers Assigned (now)` answer "what is this coordinator process doing right
  now?" from `/api/v1/jobs/current`.
- Numeric `phase_code` keeps Grafana stat cards from dropping live string
  values and falling back to `idle`.

The lifecycle section has one panel:

- `Phase Share by Proof (last 5 terminal jobs)`: horizontal stacked duration
  chart for quick phase-cost scanning. Use `Recent Proof History` for full IDs
  and failure reasons.

## Cutover criteria

- `make validate` passes (regenerates `known-contract.json`, builds jsonnet,
  runs `zisk-coordinator-dashboard-validator`)
- `make smoke` against a running Grafana 11.2 passes
- `make integration` passes during a real proof run and proves the terminal job
  appears through Grafana's Postgres-backed history panels
- Manual visual check in real Grafana
