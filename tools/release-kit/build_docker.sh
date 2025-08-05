#!/bin/bash

set -e

IMAGE_NAME="zisk-release-kit"

echo "🔨 Building Docker image for ZisK release kit..."
docker build -t ${IMAGE_NAME}:latest .
echo "📦 Docker image '${IMAGE_NAME}' built successfully."