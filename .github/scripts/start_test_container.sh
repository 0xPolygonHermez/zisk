#!/usr/bin/env bash
# Start the ZisK test container with systemd support.
#
# The container runs unprivileged: systemd only needs CAP_SYS_ADMIN, a private
# cgroup namespace (cgroup v2) and relaxed seccomp/AppArmor profiles.
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

# systemd as PID 1 requires cgroup v2 on the host.
if [[ "$(stat -fc %T /sys/fs/cgroup 2>/dev/null)" != "cgroup2fs" ]]; then
    echo "ERROR: host is not running cgroup v2; systemd cannot run in an unprivileged container" >&2
    exit 1
fi

# The bash wrapper remounts the container's private cgroup2 subtree rw when
# Docker mounted it read-only: systemd needs it writable.
docker run -d -t \
    --name "${TEST_CONTAINER}" \
    --pull=always \
    --cgroupns=private \
    --cap-add SYS_ADMIN \
    -e container=docker \
    --security-opt seccomp=unconfined \
    --security-opt apparmor=unconfined \
    --tmpfs /run \
    --tmpfs /run/lock \
    --tmpfs /tmp \
    "${GPU_ARGS[@]}" \
    --shm-size=48g \
    -v "$GITHUB_WORKSPACE":/workspace/zisk:rw \
    -v /home/gha/cache-setup:/home/ziskuser/output:rw \
    -e ZISK_GHA=1 \
    -e ZISK_CI_NO_DOCKER_REBUILD=1 \
    -e ZISK_REPO_DIR=/workspace/zisk \
    -e WORKSPACE_DIR=/workspace \
    -e PROVE_FLAGS=-y \
    -e TERM=xterm \
    "${IMAGE}" \
    /bin/bash -c 'if [ ! -w /sys/fs/cgroup ]; then mount -t cgroup2 cgroup2 /sys/fs/cgroup; fi; exec /sbin/init'

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

# The in-container Docker daemon is unused and cannot start unprivileged; mask it.
docker exec "${TEST_CONTAINER}" bash -lc '
    systemctl mask --now docker.service docker.socket containerd.service 2>/dev/null || true
'

docker exec "${TEST_CONTAINER}" bash -lc '
    echo "PID 1:"
    ps -p 1 -o pid,comm,args
    systemctl is-system-running || true
'

# Fail fast if systemd did not come up.
echo "Waiting for systemd to settle..."
for i in $(seq 1 30); do
    state=$(docker exec "${TEST_CONTAINER}" systemctl is-system-running 2>/dev/null || true)
    # "degraded" (some unit failed) still leaves systemd fully operational.
    if [[ "$state" == "running" || "$state" == "degraded" ]]; then
        echo "systemd is ready (state: $state)."
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "systemd did not become ready in time (state: ${state:-unknown})"
        docker exec "${TEST_CONTAINER}" systemctl --failed --no-pager || true
        docker exec "${TEST_CONTAINER}" journalctl --no-pager -n 50 || true
        exit 1
    fi
    sleep 2
done
