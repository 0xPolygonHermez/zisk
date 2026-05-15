#!/usr/bin/env bash
# Wrapper for 'docker exec' with cancellation support.
# When SIGINT/SIGTERM is received (e.g. from a GitHub Actions job cancellation),
# it forcefully removes the container so the runner step exits immediately
# instead of blocking until the docker exec command finishes on its own.
#
# Usage: docker_exec.sh CONTAINER [docker exec flags/args...]
#   e.g. docker_exec.sh my-container -u myuser bash -lc 'echo hello'
set -e
CONTAINER="$1"
shift

_cleanup() {
    echo "Signal received — stopping container ${CONTAINER}..."
    docker rm -f "${CONTAINER}" 2>/dev/null || true
}
trap _cleanup INT TERM

docker exec "${CONTAINER}" "$@" &
wait $!
