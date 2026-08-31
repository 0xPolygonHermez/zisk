#!/usr/bin/env bash
# Start the ZisK test container with systemd support and wait for the
# in-container Docker daemon to be ready.
#
# The container name is read from the TEST_CONTAINER environment variable.
# The image can be overridden with the IMAGE environment variable.
#
# Usage: start_test_container.sh
set -e

if [[ -z "${TEST_CONTAINER:-}" ]]; then
    echo "ERROR: TEST_CONTAINER environment variable is not set" >&2
    exit 1
fi

if [[ -z "${GITHUB_WORKSPACE:-}" ]]; then
    echo "ERROR: GITHUB_WORKSPACE environment variable is not set" >&2
    exit 1
fi

IMAGE="${IMAGE:-ziskvm/zisk-runner-gpu:latest}"

# GPU access is requested unless RUNNER_WITH_GPU is set to 0: a runner without a
# GPU has no NVIDIA container runtime, and 'docker run --gpus all' fails
# outright there. Such a runner also needs IMAGE=ziskvm/zisk-runner:latest, the
# image built without CUDA.
GPU_ARGS=()
if [[ "${RUNNER_WITH_GPU:-1}" == "1" ]]; then
    GPU_ARGS+=(--gpus all)
fi

docker rm -f "${TEST_CONTAINER}" || true

docker run -d \
    --name "${TEST_CONTAINER}" \
    --pull=always \
    --privileged \
    --cgroupns=host \
    "${GPU_ARGS[@]}" \
    --shm-size=48g \
    -v /sys/fs/cgroup:/sys/fs/cgroup:rw \
    -v "$GITHUB_WORKSPACE":/workspace/zisk:rw \
    -v /home/gha/cache-setup:/home/ziskuser/output:rw \
    -e ZISK_GHA=1 \
    -e ZISK_REPO_DIR=/workspace/zisk \
    -e WORKSPACE_DIR=/workspace \
    -e PROVE_FLAGS=-y \
    -e TERM=xterm \
    "${IMAGE}" \
    /sbin/init

sleep 3

if ! docker ps --format '{{.Names}}' | grep -q "^${TEST_CONTAINER}$"; then
    echo "Container stopped unexpectedly"
    docker ps -a
    docker logs "${TEST_CONTAINER}" || true
    docker inspect "${TEST_CONTAINER}" --format '{{.State.ExitCode}} {{.State.Error}}' || true
    exit 1
fi

# /workspace is created root-owned by Docker for the bind mount at /workspace/zisk.
# WORKSPACE_DIR is set to /workspace so the build scripts can clone sibling repos there.
# Make it writable by ziskuser (non-recursive, so the mounted /workspace/zisk keeps its ownership)
# so the build scripts can clone/build the sibling repos (zisk-ethproofs, zisk-eth-client) there.
docker exec "${TEST_CONTAINER}" chown ziskuser:ziskuser /workspace

docker exec "${TEST_CONTAINER}" bash -lc '
    echo "PID 1:"
    ps -p 1 -o pid,comm,args
    systemctl is-system-running || true
'

# Wait for the in-container Docker daemon (started by systemd) to be ready.
# lib-float/build.rs invokes `docker` early in the build, so it must be up
# before the "Build ZisK" step runs.
echo "Waiting for in-container Docker daemon..."
for i in $(seq 1 30); do
    if docker exec -u ziskuser "${TEST_CONTAINER}" docker info >/dev/null 2>&1; then
        echo "Docker daemon is ready."
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "Docker daemon did not become ready in time"
        docker exec "${TEST_CONTAINER}" systemctl status docker.service --no-pager || true
        docker exec "${TEST_CONTAINER}" journalctl -u docker.service --no-pager -n 50 || true
        exit 1
    fi
    sleep 2
done
