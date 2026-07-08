#!/bin/bash

set -e

IMAGE_NAME="zisk-runpod"

# Resolve paths relative to this script so it works from any directory.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKERFILE="${SCRIPT_DIR}/Dockerfile"

# The Dockerfile copies utils.sh + install_deps.sh, which live in tools/test-env,
# so that folder must be the build context.
BUILD_CONTEXT="${SCRIPT_DIR}/../test-env"

echo "Building Docker image ${IMAGE_NAME}..."
docker build -f "${DOCKERFILE}" -t "${IMAGE_NAME}:latest" "${BUILD_CONTEXT}"
echo "Docker image '${IMAGE_NAME}' built successfully."
