#!/usr/bin/env bash
# Build lib/ziskfloat.elf and lib/libziskfloat.a in the canonical docker image.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
C_DIR="$(dirname -- "$SCRIPT_DIR")"

IMAGE_TAG="$("$SCRIPT_DIR/ensure-base-image.sh")"

if docker info >/dev/null 2>&1; then
    DOCKER=(docker)
elif command -v sudo >/dev/null 2>&1; then
    DOCKER=(sudo docker)
else
    echo "ERROR: need docker (with group access) or sudo." >&2
    exit 1
fi

# Honour SUDO_UID/GID so outputs are owned by the invoking user even when the
# script itself is run under sudo.
HOST_UID="${SUDO_UID:-$(id -u)}"
HOST_GID="${SUDO_GID:-$(id -g)}"

echo "==> Rebuilding lib/ziskfloat.elf and lib/libziskfloat.a in $IMAGE_TAG"
"${DOCKER[@]}" run --rm \
    --user "$HOST_UID:$HOST_GID" \
    --volume "$C_DIR":/work \
    --workdir /work \
    --env SOURCE_DATE_EPOCH=0 \
    "$IMAGE_TAG" \
    bash -c 'make clean && make'

echo "==> Done. Artifacts:"
sha256sum "$C_DIR/lib/ziskfloat.elf" "$C_DIR/lib/libziskfloat.a"
