#!/bin/bash

set -e

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"

IMAGE_NAME="zisk-test-env"

# Check if --gpu was passed
BUILD_ARGS=""
if [[ "$1" == "--gpu" ]]; then
    BUILD_ARGS="--build-arg CUDA=true"
    IMAGE_NAME="${IMAGE_NAME}-gpu"
fi

BASE_IMAGE="$("$REPO_ROOT/lib-float/c/scripts/ensure-base-image.sh")"

echo "Building Docker image ${IMAGE_NAME}..."
docker build --build-arg BASE_IMAGE="$BASE_IMAGE" ${BUILD_ARGS} -t ${IMAGE_NAME}:latest "$SCRIPT_DIR"
echo "Docker image '${IMAGE_NAME}' built successfully."
