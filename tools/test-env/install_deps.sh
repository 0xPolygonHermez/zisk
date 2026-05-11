#!/bin/bash

source ./utils.sh

# ensure_sudo: Ensure the command is run with sudo if necessary
ensure_sudo() {
    if [[ "$(id -u)" -ne 0 ]]; then
        sudo -- "$@" || return 1
    else
        "$@" || return 1
    fi
}

# install_cuda: Install NVIDIA CUDA toolkit on Linux
# Parameters:
#   $1 (optional) — CUDA version "X-Y" (e.g. "12-1"). Defaults to "12-1".
install_cuda() {
    local CUDA_VER="${1:-12-1}"

    # Derive distro string from /etc/os-release (e.g. ubuntu2204, ubuntu2404, debian12)
    local distro
    distro="$(. /etc/os-release && echo "${ID}${VERSION_ID//./}")"

    echo "🔧 Installing NVIDIA CUDA toolkit ${CUDA_VER} for ${distro}..."

    ensure_sudo apt-get update || return 1
    ensure_sudo apt-get install -y --no-install-recommends gnupg2 curl ca-certificates software-properties-common || return 1

    ensure wget "https://developer.download.nvidia.com/compute/cuda/repos/${distro}/x86_64/cuda-keyring_1.1-1_all.deb" || return 1

    ensure_sudo dpkg -i cuda-keyring_1.1-1_all.deb || return 1
    ensure_sudo apt-get update || return 1

    ensure_sudo apt-get install -y cuda-toolkit-${CUDA_VER} || return 1
}

# install_dependencies_linux: Install package dependencies for Linux
install_dependencies_linux() {
    current_step=1

    # Check if --cuda argument was passed
    INSTALL_CUDA=false
    for arg in "$@"; do
        if [[ "$arg" == "--cuda" ]]; then
            INSTALL_CUDA=true
            break
        fi
    done

    if [[ "$INSTALL_CUDA" == true ]]; then
        total_steps=5
    else
        total_steps=4
    fi

    step "Installing package dependencies for linux x86_64..."

    ensure_sudo apt-get update || return 1
    ensure_sudo apt-get install -y apt-utils dialog libterm-readline-perl-perl || return 1

    ensure_sudo apt-get install -y curl git xz-utils jq build-essential qemu-system libomp-dev libgmp-dev \
        nlohmann-json3-dev protobuf-compiler uuid-dev libgrpc++-dev libsecp256k1-dev \
        libsodium-dev libpqxx-dev nasm libopenmpi-dev openmpi-bin openmpi-common \
        sudo ca-certificates gnupg lsb-release wget libclang-dev clang gcc-riscv64-unknown-elf || return 1

    step "Installing Node.js 20.x..."
    curl -fsSL https://deb.nodesource.com/setup_20.x | ( [[ "$(id -u)" -ne 0 ]] && sudo -E bash || bash )
    ensure_sudo apt-get install -y nodejs || return 1

    step "Installing Rust..."
    # Create the profile file if it doesn't exist
#    touch $PROFILE
#    ensure curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || return 1
#    export PATH="${HOME}/.cargo/bin:$PATH"
#    source "${HOME}/.cargo/env"

    if [[ "$INSTALL_CUDA" == true ]]; then
        step "Installing CUDA..."
        install_cuda 12-9 || return 1
    fi

    step "Installing nano editor..."
    ensure_sudo apt-get install -y nano || return 1
}

# install_dependencies_darwin: Install package dependencies for macOS
install_dependencies_darwin() {
    current_step=1
    total_steps=1

    step "Installing package dependencies for macOS..."
    ensure brew reinstall jq curl libomp protobuf openssl nasm pkgconf open-mpi libffi nlohmann-json libsodium || return 1
}

main() {
    info "▶️ Running $(basename "$0") script..."

    # Check the system type and call the respective function
    if [[ "${PLATFORM}" == "linux" ]]; then
        install_dependencies_linux "$@" || return 1
    elif [[ "${PLATFORM}" == "darwin" ]]; then
        install_dependencies_darwin "$@" || return 1
    else
        err "unsupported OS"
        return 1
    fi
}

main "$@"
