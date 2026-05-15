#!/bin/bash

set -e

IMAGE_NAME="zisk-test-env"
PUSH_IMAGE="ziskvm/zisk-runner"

# Check if --gpu was passed
if [[ "$1" == "--gpu" ]]; then
    IMAGE_NAME="${IMAGE_NAME}-gpu"
    PUSH_IMAGE="${PUSH_IMAGE}-gpu"
fi

echo "Pushing Docker image ${PUSH_IMAGE}..."
docker tag ${IMAGE_NAME}:latest ${PUSH_IMAGE}:latest
docker push ${PUSH_IMAGE}:latest