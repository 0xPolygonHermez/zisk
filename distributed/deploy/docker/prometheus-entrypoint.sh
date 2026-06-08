#!/bin/sh
# Renders Prom config from env. ZISK_COORDS is preferred:
#   ZISK_COORDS="coord-mac-2=host.docker.internal:29090,coord-vast-1=host.docker.internal:19090"
# Legacy single-coord vars (COORD_HOST + COORD_ID) still work for back-compat
# and are auto-translated into a one-entry ZISK_COORDS list.

set -eu

if [ -z "${ZISK_COORDS:-}" ]; then
    : "${COORD_HOST:?ZISK_COORDS or COORD_HOST required}"
    : "${COORD_ID:?ZISK_COORDS or COORD_ID required}"
    ZISK_COORDS="${COORD_ID}=${COORD_HOST}"
fi

# Build one scrape job per coord entry. Indentation must match the template
# (two-space block under scrape_configs).
SCRAPE_BLOCK=""
OLD_IFS="$IFS"
IFS=','
for entry in $ZISK_COORDS; do
    IFS="$OLD_IFS"
    id="${entry%%=*}"
    host="${entry#*=}"
    if [ -z "$id" ] || [ -z "$host" ] || [ "$id" = "$entry" ]; then
        echo "ERROR: malformed ZISK_COORDS entry '$entry' (expected id=host:port)" >&2
        exit 2
    fi
    SCRAPE_BLOCK="${SCRAPE_BLOCK}  - job_name: zisk-coordinator-${id}
    honor_labels: true
    static_configs:
      - targets: [\"${host}\"]
        labels:
          coordinator_id: \"${id}\"
    metrics_path: /metrics
    authorization:
      type: Bearer
      credentials_file: /etc/prometheus/zisk-scrape-token
"
    IFS=','
done
IFS="$OLD_IFS"

awk -v block="$SCRAPE_BLOCK" '{
    gsub(/\$\{COORD_SCRAPE_CONFIGS\}/, block)
    print
}' /etc/prometheus/prometheus.local.yml.tpl > /etc/prometheus/prometheus.yml

exec /bin/prometheus \
    --config.file=/etc/prometheus/prometheus.yml \
    --storage.tsdb.path=/prometheus \
    --web.console.libraries=/etc/prometheus/console_libraries \
    --web.console.templates=/etc/prometheus/consoles
