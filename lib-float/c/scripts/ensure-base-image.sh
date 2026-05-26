#!/usr/bin/env bash
# Build the canonical ziskfloat docker image (idempotent) and print its tag on stdout.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
C_DIR="$(dirname -- "$SCRIPT_DIR")"

# Tag includes the apt pin so a pin bump can't alias on top of a stale image.
ZISKFLOAT_BASE_IMAGE="${ZISKFLOAT_BASE_IMAGE:-ziskfloat-build:ubuntu24.04-gcc13.2.0-11ubuntu1_12}"

if docker info >/dev/null 2>&1; then
    DOCKER=(docker)
elif command -v sudo >/dev/null 2>&1; then
    echo "ℹ docker daemon not accessible to $(id -un); using sudo (may prompt for password)" >&2
    DOCKER=(sudo docker)
else
    echo "ERROR: need docker (with group access) or sudo." >&2
    exit 1
fi

"${DOCKER[@]}" build \
    --file "$C_DIR/Dockerfile.build" \
    --tag "$ZISKFLOAT_BASE_IMAGE" \
    "$C_DIR" >&2

echo "$ZISKFLOAT_BASE_IMAGE"
