#!/usr/bin/env bash
# Start a bare container: a plain OS image with no ZisK dependencies and no
# ZisK bundle preinstalled.
#
# Unlike start_test_container.sh (which uses the prebuilt zisk-runner-gpu
# image), nothing is installed here: installing the dependencies and the ziskup
# bundle from scratch is exactly what the caller is meant to exercise. No
# systemd (nor an in-container Docker daemon) is needed either, so the
# container just idles while each workflow step runs through 'docker exec'.
#
# The container name is read from the TEST_CONTAINER environment variable.
# The image can be overridden with the IMAGE environment variable.
#
# Usage: start_bare_container.sh
set -e

if [[ -z "${TEST_CONTAINER:-}" ]]; then
    echo "ERROR: TEST_CONTAINER environment variable is not set" >&2
    exit 1
fi

IMAGE="${IMAGE:-ubuntu:22.04}"

# PATH is set on the container (and the workflow steps use 'bash -c', not
# 'bash -lc') so the toolchains installed by rustup and ziskup are visible to
# every step without /etc/profile resetting PATH on login shells.
CONTAINER_PATH="/root/.cargo/bin:/root/.zisk/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

# GPU access is requested unless RUNNER_WITH_GPU is set to 0: a runner without a
# GPU has no NVIDIA container runtime, and 'docker run --gpus all' fails
# outright there.
GPU_ARGS=()
if [[ "${RUNNER_WITH_GPU:-1}" == "1" ]]; then
    GPU_ARGS+=(--gpus all)
fi

# Forward the build tuning variables set at job level, when present.
ENV_ARGS=()
for var in CARGO_BUILD_JOBS CARGO_INCREMENTAL; do
    if [[ -n "${!var:-}" ]]; then
        ENV_ARGS+=(-e "${var}=${!var}")
    fi
done

docker rm -f "${TEST_CONTAINER}" || true

# SHELL is set because ziskup aborts when it cannot detect the shell from it,
# and 'docker exec' provides no SHELL by default.
docker run -d \
    --name "${TEST_CONTAINER}" \
    --pull=always \
    --privileged \
    "${GPU_ARGS[@]}" \
    --shm-size=48g \
    -e PATH="${CONTAINER_PATH}" \
    -e SHELL=/bin/bash \
    -e HOME=/root \
    -e DEBIAN_FRONTEND=noninteractive \
    -e TERM=xterm \
    "${ENV_ARGS[@]}" \
    "${IMAGE}" \
    sleep infinity

sleep 3

if ! docker ps --format '{{.Names}}' | grep -q "^${TEST_CONTAINER}$"; then
    echo "Container stopped unexpectedly"
    docker ps -a
    docker logs "${TEST_CONTAINER}" || true
    docker inspect "${TEST_CONTAINER}" --format '{{.State.ExitCode}} {{.State.Error}}' || true
    exit 1
fi

docker exec "${TEST_CONTAINER}" bash -c 'cat /etc/os-release | head -2; echo "PATH=$PATH"'
