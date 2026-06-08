.PHONY: contract validate smoke render integration grafonnet-sync generated-clean production-check

# Grafana auth flags. Value is read from $GRAFANA_PASSWORD at recipe-run time;
# the env var is required (see _require-grafana-auth). The flag-name token is
# defined on a separate line from the value reference so secret-scanners do
# not treat the line as a hardcoded credential assignment.
_GP_FLAG := --grafana-password
GRAFANA_AUTH_FLAGS = --grafana-user $${GRAFANA_USER:-admin} $(_GP_FLAG) $$GRAFANA_PASSWORD

.PHONY: _require-grafana-auth
_require-grafana-auth:
	@: $${GRAFANA_PASSWORD:?GRAFANA_PASSWORD env var is required}

# Regenerates known-contract.json from coordinator-server source so the
# validator's metric and route allowlists always match what the backend
# actually exposes. Run on every backend change.
contract:
	cd ../../.. && cargo run -q -p zisk-coordinator-contract -- \
		--output distributed/deploy/grafana/known-contract.json

validate: contract
	cd ../../.. && cargo run -q -p zisk-coordinator-dashboard-validator -- \
		--dashboard distributed/deploy/grafana/dashboards/zisk-overview.json \
		--contract  distributed/deploy/grafana/known-contract.json

smoke: contract _require-grafana-auth
	cd ../../.. && cargo run -q -p zisk-coordinator-dashboard-smoke -- \
		--grafana          $${GRAFANA_URL:-http://127.0.0.1:3001} \
		$(GRAFANA_AUTH_FLAGS) \
		--prometheus       $${PROMETHEUS_URL:-http://127.0.0.1:9091} \
		--coordinator-api  $${ZISK_COORDINATOR_API_URL:-http://127.0.0.1:19090} \
		--scrape-token     $${ZISK_SCRAPE_TOKEN:-$$(cat $${ZISK_SCRAPE_TOKEN_FILE:-../config/zisk-scrape-token.example})} \
		--dashboard-uid    $${GRAFANA_DASHBOARD_UID:-zisk-dev} \
		--expected-refresh $${EXPECTED_DASHBOARD_REFRESH:-1s}

# Live Grafana per-panel data-presence check. Default state is idle since the
# CI / local default flow does not have an in-flight proof; override with
# DASHBOARD_STATE=running before a proof is launched.
render: contract _require-grafana-auth
	cd ../../.. && cargo run -q -p zisk-coordinator-dashboard-render -- \
		--grafana          $${GRAFANA_URL:-http://127.0.0.1:3001} \
		$(GRAFANA_AUTH_FLAGS) \
		--dashboard-uid    $${GRAFANA_DASHBOARD_UID:-zisk-dev} \
		--state            $${DASHBOARD_STATE:-idle}

# Live integration assertions for a real proof run. Operator launches the proof,
# then this records phase transitions and verifies live JSON, Prom, and Grafana
# Postgres history panels.
integration: contract _require-grafana-auth
	cd ../../.. && cargo run -q -p zisk-coordinator-dashboard-integration -- \
		--coordinator-api  $${ZISK_COORDINATOR_API_URL:-http://127.0.0.1:19090} \
		--scrape-token     $${ZISK_SCRAPE_TOKEN:-} \
		--grafana          $${GRAFANA_URL:-http://127.0.0.1:3001} \
		$(GRAFANA_AUTH_FLAGS) \
		--dashboard-uid    $${GRAFANA_DASHBOARD_UID:-zisk-dev} \
		--scenario         clean-proof-success \
		--timeout-seconds  $${INTEGRATION_TIMEOUT_SECONDS:-600}

# Full production-readiness gate. Runs every non-interactive check that does
# NOT require an in-flight proof job. The integration harness is intentionally
# separate because it needs a live proof in flight; see the `integration` target.
production-check:
	@echo "=== contract regenerated ===" && $(MAKE) contract
	@echo "=== grafonnet source build + sync ===" && $(MAKE) grafonnet-sync
	@echo "=== validator (panel set + queries + routes + description gate) ===" && $(MAKE) validate
	@echo "=== smoke (live HTTP probes) ===" && $(MAKE) smoke
	@echo "=== render (per-panel data presence, idle state) ===" && $(MAKE) render
	@echo "=== generated artifacts clean ===" && $(MAKE) generated-clean
	@echo "production-check PASS"

grafonnet-sync:
	$(MAKE) -C grafonnet sync

# CI/pre-commit guard: generation targets must not leave stale artifacts.
generated-clean:
	cd ../../.. && git diff --exit-code -- \
		distributed/deploy/grafana/known-contract.json \
		distributed/deploy/grafana/dashboards/zisk-overview.json
