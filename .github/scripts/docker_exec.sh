#!/usr/bin/env bash
# Wrapper for 'docker exec' with cancellation support.
# When SIGINT/SIGTERM is received (e.g. from a GitHub Actions job cancellation),
# it forcefully removes the container so the runner step exits immediately
# instead of blocking until the docker exec command finishes on its own.
#
# The container name is read from the TEST_CONTAINER environment variable.
#
# Usage: docker_exec.sh [docker exec options] COMMAND [ARGS...]
#   e.g. docker_exec.sh -u myuser -e VAR=val bash -lc 'echo hello'
set -e

if [[ -z "${TEST_CONTAINER:-}" ]]; then
    echo "ERROR: TEST_CONTAINER environment variable is not set" >&2
    exit 1
fi

_cleanup() {
    echo "Signal received — stopping container ${TEST_CONTAINER}..."
    docker rm -f "${TEST_CONTAINER}" 2>/dev/null || true
}
trap _cleanup INT TERM

docker exec "${TEST_CONTAINER}" "$@" &
wait $!
