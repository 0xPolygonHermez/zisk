#!/bin/bash

set -e

# Check if --gpu was passed
DOCKER_IMAGE="zisk-test-env"
if [[ "$1" == "--gpu" ]]; then
    DOCKER_IMAGE="${DOCKER_IMAGE}-gpu"
fi

echo "Pushing Docker image ${DOCKER_IMAGE}..."
docker tag zisk-test-env:latest ziskvm/${DOCKER_IMAGE}:latest
docker push ziskvm/${DOCKER_IMAGE}:latest