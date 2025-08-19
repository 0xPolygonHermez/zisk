#!/bin/bash

set -e

IMAGE_NAME="zisk-test-env"

# Check if --gpu was passed
BUILD_ARGS=""
if [[ "$1" == "--gpu" ]]; then
    BUILD_ARGS="--build-arg GPU=true"
    IMAGE_NAME="${IMAGE_NAME}-gpu"
fi

echo "Building Docker image ${IMAGE_NAME}..."
docker build ${BUILD_ARGS} -t ${IMAGE_NAME}:latest .
echo "Docker image '${IMAGE_NAME}' built successfully."
