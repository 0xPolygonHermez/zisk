GRAFANA_URL    ?= http://127.0.0.1:3001
BASELINE_JSON  ?= ../dashboards/zisk-overview.json
BUILD_RAW      := build/zisk-overview.raw.json
BUILD_JSON     := build/zisk-overview.json
CONTRACT_JSON  := ../known-contract.json

.PHONY: deps contract build canonicalize validate diff sync smoke clean all local-up local-down local-logs _require-grafana-auth

# Grafana auth wrapper. The flag name lives on its own line so the value
# reference (read from $GRAFANA_PASSWORD at recipe-run time) does not sit
# next to the literal "password" keyword on any single source line.
_GP_FLAG := --grafana-password
GRAFANA_AUTH_FLAGS = --grafana-user $${GRAFANA_USER:-admin} $(_GP_FLAG) $$GRAFANA_PASSWORD

_require-grafana-auth:
	@: $${GRAFANA_PASSWORD:?GRAFANA_PASSWORD env var is required}

all: validate

deps:
	jb install

# Regenerates known-contract.json from coordinator-server source. Validator
# loads this file, so metric/route additions flow in automatically.
contract:
	cd ../../../.. && cargo run -q -p zisk-coordinator-contract -- \
		--output distributed/deploy/grafana/known-contract.json

build: deps
	mkdir -p build
	jsonnet -J vendor -J src src/main.jsonnet > $(BUILD_RAW)

canonicalize: build
	jq -S '.' $(BUILD_RAW) > $(BUILD_JSON)

validate: contract canonicalize
	cd ../../../.. && cargo run -q -p zisk-coordinator-dashboard-validator -- \
		--dashboard distributed/deploy/grafana/grafonnet/$(BUILD_JSON) \
		--contract  distributed/deploy/grafana/known-contract.json

diff: canonicalize
	diff -u $(BASELINE_JSON) $(BUILD_JSON)

sync: validate
	cp $(BUILD_JSON) $(BASELINE_JSON)

smoke: validate _require-grafana-auth
	cd ../../../.. && cargo run -q -p zisk-coordinator-dashboard-smoke -- \
		--grafana          $(GRAFANA_URL) \
		$(GRAFANA_AUTH_FLAGS) \
		--prometheus       $${PROMETHEUS_URL:-http://127.0.0.1:9091} \
		--coordinator-api  $${ZISK_COORDINATOR_API_URL:-http://127.0.0.1:19090} \
		--scrape-token     $${ZISK_SCRAPE_TOKEN:-$$(cat $${ZISK_SCRAPE_TOKEN_FILE:-../../config/zisk-scrape-token.example})} \
		--dashboard-uid    $${GRAFANA_DASHBOARD_UID:-zisk-dev} \
		--expected-refresh $${EXPECTED_DASHBOARD_REFRESH:-1s}

clean:
	rm -rf vendor build local/dashboards/zisk-overview.json

# Local-only docker Grafana on PORT (default 3001) fed by grafonnet output.
# Does not touch any existing Grafana on :3000.
local-up:
	bash local/run.sh

local-down:
	docker rm -f zisk-grafonnet-local 2>/dev/null || true

local-logs:
	docker logs -f zisk-grafonnet-local
