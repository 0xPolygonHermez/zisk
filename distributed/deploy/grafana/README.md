# ZisK Grafana Provisioning

The dashboard JSON is environment-neutral. It must not embed coordinator,
Prometheus, localhost, or Docker host URLs.

Provisioning owns runtime endpoints:

- `ZISK_PROMETHEUS_URL` configures the `prometheus` datasource.
- `ZISK_COORDINATOR_API_URL` configures the `zisk-json` Infinity datasource.
- The `zisk-json` bearer token is read from `/etc/grafana/secrets/zisk-scrape-token`
  (mounted by compose / `local/run.sh` from the same on-disk file Prometheus
  uses, so the two services never drift).

Dashboard panels that read coordinator JSON use relative paths such as
`/api/v1/jobs/current` and `/api/v1/workers`. Grafana resolves those paths
through the `zisk-json` datasource base URL. Historical dashboard panels use the
provisioned `zisk-postgres` datasource instead of the deprecated coordinator
history JSON endpoints.

For docker compose, defaults are set in `distributed/deploy/docker/compose.yaml`.
For local Homebrew Grafana, set the same environment variables before starting
Grafana or provision equivalent datasources with the same pinned UIDs:

- `prometheus`
- `zisk-json`

The canonical dashboard UID is `zisk-dev`, so the local URL is:

`http://127.0.0.1:3000/d/zisk-dev/zisk-coordinator`

Do not fix dashboard connectivity by editing the Grafana SQLite database. That
is local state and is not a deployable source of truth.

Before handing off dashboard changes, run:

```bash
make validate
```

The validator is intentionally strict. It rejects v0.17 worker metrics that the
v0.18 coordinator no longer emits, hardcoded local URLs, high-cardinality
Prometheus labels such as `job_id`/`hash_id`, and active-proof tables that omit
the operator fields needed to answer where a proof is headed.

For a live local stack, also run:

```bash
GRAFANA_PASSWORD=... ZISK_SCRAPE_TOKEN=... make smoke
```

The smoke check verifies the deployed contract: Grafana is serving the expected
dashboard, Prometheus can see the coordinator, the live coordinator JSON
endpoints respond, and the Postgres-backed history datasource exposes the
operator fields used by recent proof tables. A static dashboard JSON can be valid
while the running coordinator is still an older binary; the smoke check catches
that explicitly.
