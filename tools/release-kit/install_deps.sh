#!/bin/bash

source ./utils.sh

# install_cuda: Add NVIDIA repo and install only CUDA & cuDNN dev packages
# Parameters:
#   $1 (optional) — CUDA version "X-Y" (e.g. "12-1"). Defaults to "12-1".
install_cuda() {
    local CUDA_VER="${1:-12-1}"
    # Build distribution string for the repo (ubuntu + version without dot)
    local distro
    distro="$(. /etc/os-release && echo "${ID}${VERSION_ID//./}")"
    local keyring="/usr/share/keyrings/nvidia-cuda-keyring.gpg"

    echo "🔧 Installing NVIDIA CUDA toolkit ${CUDA_VER} for ${distro}..."

    # Basic deps
    apt-get update -qq
    ensure apt-get install -y --no-install-recommends \
        gnupg2 curl ca-certificates software-properties-common

    # Import NVIDIA GPG key into a keyring
    curl -fsSL \
        "https://developer.download.nvidia.com/compute/cuda/repos/${distro}/x86_64/3bf863cc.pub" \
      | gpg --dearmor --yes \
      | tee "${keyring}"

    # Add CUDA repo signed by our keyring
    echo \
      "deb [signed-by=${keyring}] https://developer.download.nvidia.com/compute/cuda/repos/${distro}/x86_64/ /" \
      | tee /etc/apt/sources.list.d/cuda.list

    # Install only the dev packages
    apt-get update -qq
    ensure apt-get install -y --no-install-recommends \
        "cuda-toolkit-${CUDA_VER}" \
        libcudnn8-dev

    # Clean up
    apt-get clean
    rm -rf /var/lib/apt/lists/*
}

main() {
    current_step=1

    # Check if --gpu argument was passed
    INSTALL_GPU=false
    for arg in "$@"; do
        if [[ "$arg" == "--gpu" ]]; then
            INSTALL_GPU=true
            break
        fi
    done

    if [[ "$INSTALL_GPU" == true ]]; then
        total_steps=4
    else
        total_steps=3
    fi

    step "Installing package dependencies..."

    apt-get update
    apt-get install -y apt-utils dialog libterm-readline-perl-perl

    ensure apt-get install -y \
        curl git xz-utils jq build-essential qemu-system libomp-dev libgmp-dev \
        nlohmann-json3-dev protobuf-compiler uuid-dev libgrpc++-dev libsecp256k1-dev \
        libsodium-dev libpqxx-dev nasm libopenmpi-dev openmpi-bin openmpi-common \
        sudo ca-certificates gnupg lsb-release wget libclang-dev clang || return 1

    step "Installing Node.js 20.x..."
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash
    ensure apt-get install -y nodejs || return 1

    step "Installing Rust..."
    # Create the profile file if it doesn't exist
    touch $PROFILE
    ensure curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || return 1
    export PATH="${HOME}/.cargo/bin:$PATH"
    source "${HOME}/.cargo/env"

    if [[ "$INSTALL_GPU" == true ]]; then
        step "Installing CUDA..."
        install_cuda 12-1 || return 1
    else
        warn "skipping CUDA installation (no --gpu flag passed)"
    fi

    step "Installing nano editor..."
    ensure apt-get install -y nano || return 1

    cat $PROFILE
}

main "$@"
