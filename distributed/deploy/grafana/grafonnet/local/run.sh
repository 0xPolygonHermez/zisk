#!/usr/bin/env bash
# Spin up a parallel Grafana on a chosen port, fed by the grafonnet-generated
# dashboard. Does NOT touch the production Grafana on :3000.
#
# Required env (no silent defaults — fails loud if missing):
#   ZISK_PROMETHEUS_URL       Prometheus base URL reachable from inside docker
#   ZISK_COORDINATOR_API_URL  Coordinator API base URL reachable from inside docker
#   ZISK_SCRAPE_TOKEN         Bearer token the running coordinator accepts
#   GRAFANA_PASSWORD          Admin password for the local Grafana container
#   ZISK_POSTGRES_PASSWORD    Password for the configured Postgres datasource
#
# Optional:
#   PORT      host port to publish Grafana on (default 3001)
#   IMAGE     Grafana image (default grafana/grafana:11.2.0, pinned to deploy)
#   ZISK_POSTGRES_DATASOURCE_HOST  Postgres host from inside docker (default host.docker.internal)
#   ZISK_POSTGRES_DATASOURCE_PORT  Postgres port from inside docker (default 15432)
#   ZISK_POSTGRES_DATASOURCE_DB    Postgres database (default zisk_history)
#   ZISK_POSTGRES_DATASOURCE_USER  Postgres user (default zisk)
#   ZISK_POSTGRES_PASSWORD         Postgres password
#
# Inside docker the host is reachable via host.docker.internal. Sourcing the
# URLs from .env or shell history is the caller's responsibility — this script
# refuses to guess.
#
# Quickstart for the running e2e local cluster:
#   ZISK_PROMETHEUS_URL=http://host.docker.internal:9091 \
#   ZISK_COORDINATOR_API_URL=http://host.docker.internal:19090 \
#   ZISK_SCRAPE_TOKEN=<scrape-token> \
#   GRAFANA_PASSWORD=<local-password> \
#   ZISK_POSTGRES_PASSWORD=<local-password> \
#   bash local/run.sh

set -euo pipefail

cd "$(dirname "$0")/.."

required=(ZISK_PROMETHEUS_URL ZISK_COORDINATOR_API_URL ZISK_SCRAPE_TOKEN GRAFANA_PASSWORD ZISK_POSTGRES_PASSWORD)
missing=()
for v in "${required[@]}"; do
  if [ -z "${!v:-}" ]; then
    missing+=("$v")
  fi
done
if [ ${#missing[@]} -gt 0 ]; then
  echo "ERROR: required env vars not set: ${missing[*]}" >&2
  echo "       see header of $0 for details" >&2
  exit 2
fi

PORT="${PORT:-3001}"
IMAGE="${IMAGE:-grafana/grafana:11.2.0}"
CONTAINER="zisk-grafonnet-local"
ZISK_POSTGRES_DATASOURCE_HOST="${ZISK_POSTGRES_DATASOURCE_HOST:-host.docker.internal}"
ZISK_POSTGRES_DATASOURCE_PORT="${ZISK_POSTGRES_DATASOURCE_PORT:-15432}"
ZISK_POSTGRES_DATASOURCE_DB="${ZISK_POSTGRES_DATASOURCE_DB:-zisk_history}"
ZISK_POSTGRES_DATASOURCE_USER="${ZISK_POSTGRES_DATASOURCE_USER:-zisk}"
ZISK_POSTGRES_DATASOURCE_SSLMODE="${ZISK_POSTGRES_DATASOURCE_SSLMODE:-disable}"

# Rebuild + stage so docker reads the latest grafonnet output.
make build canonicalize >/dev/null
cp build/zisk-overview.json local/dashboards/zisk-overview.json

# Grafana datasource provisioning reads the bearer via $__file{} from
# /etc/grafana/secrets/zisk-scrape-token. Write the in-shell ZISK_SCRAPE_TOKEN
# to a transient file under local/ so the container can mount the same
# on-disk source of truth Prometheus uses.
SECRET_DIR="$(pwd)/local/secrets"
mkdir -p "$SECRET_DIR"
printf '%s' "$ZISK_SCRAPE_TOKEN" > "$SECRET_DIR/zisk-scrape-token"
chmod 600 "$SECRET_DIR/zisk-scrape-token"

docker rm -f "$CONTAINER" >/dev/null 2>&1 || true

docker run -d \
  --name "$CONTAINER" \
  -p "${PORT}:3000" \
  -e GF_SECURITY_ADMIN_PASSWORD="$GRAFANA_PASSWORD" \
  -e GF_DASHBOARDS_MIN_REFRESH_INTERVAL=1s \
  -e GF_INSTALL_PLUGINS=yesoreyeram-infinity-datasource \
  -e ZISK_PROMETHEUS_URL="$ZISK_PROMETHEUS_URL" \
  -e ZISK_COORDINATOR_API_URL="$ZISK_COORDINATOR_API_URL" \
  -e ZISK_POSTGRES_DATASOURCE_HOST="$ZISK_POSTGRES_DATASOURCE_HOST" \
  -e ZISK_POSTGRES_DATASOURCE_PORT="$ZISK_POSTGRES_DATASOURCE_PORT" \
  -e ZISK_POSTGRES_DATASOURCE_DB="$ZISK_POSTGRES_DATASOURCE_DB" \
  -e ZISK_POSTGRES_DATASOURCE_USER="$ZISK_POSTGRES_DATASOURCE_USER" \
  -e ZISK_POSTGRES_DATASOURCE_SSLMODE="$ZISK_POSTGRES_DATASOURCE_SSLMODE" \
  -e ZISK_POSTGRES_PASSWORD="$ZISK_POSTGRES_PASSWORD" \
  -v "$(pwd)/local/provisioning:/etc/grafana/provisioning:ro" \
  -v "$(pwd)/local/dashboards:/etc/grafana/dashboards:ro" \
  -v "$SECRET_DIR/zisk-scrape-token:/etc/grafana/secrets/zisk-scrape-token:ro" \
  "$IMAGE" >/dev/null

cat <<EOF
Grafana (grafonnet) -> http://127.0.0.1:${PORT}/d/zisk-dev/zisk-coordinator
  user: admin
  Prometheus      (inside container): $ZISK_PROMETHEUS_URL
  Coordinator API (inside container): $ZISK_COORDINATOR_API_URL
  Postgres        (inside container): ${ZISK_POSTGRES_DATASOURCE_HOST}:${ZISK_POSTGRES_DATASOURCE_PORT}/${ZISK_POSTGRES_DATASOURCE_DB}
  Scrape token len: ${#ZISK_SCRAPE_TOKEN}

logs:  docker logs -f $CONTAINER
stop:  docker rm -f $CONTAINER
EOF
