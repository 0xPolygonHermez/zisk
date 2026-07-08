#!/bin/bash

set -e

IMAGE_NAME="zisk-runpod"
PUSH_IMAGE="ziskvm/zisk-runpod"

echo "Pushing Docker image ${PUSH_IMAGE}..."
docker tag ${IMAGE_NAME}:latest ${PUSH_IMAGE}:latest
docker push ${PUSH_IMAGE}:latest
