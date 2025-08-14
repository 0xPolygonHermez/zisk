#!/bin/bash

source ./utils.sh

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    total_steps=8

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 0

    cd "${WORKSPACE_DIR}"

    step  "Cloning pil2-compiler, pil2-proofman and pil2-proofman-js repos..."

    # Remove existing directories if they exist
    rm -rf pil2-compiler
    rm -rf pil2-proofman
    rm -rf pil2-proofman-js

    # Clone pil2-compiler
    ensure git clone https://github.com/0xPolygonHermez/pil2-compiler.git || return 1
    cd pil2-compiler
    # If PIL2_COMPILER_BRANCH is defined, check out the specified branch
    if [[ -n "$PIL2_COMPILER_BRANCH" ]]; then
        echo "Checking out branch '$PIL2_COMPILER_BRANCH' for pil2-compiler..."
        ensure git checkout "$PIL2_COMPILER_BRANCH" || return 1
    fi
    rm -rf package-lock.json
    rm -rf node_modules
    cd ..

    ensure git clone https://github.com/0xPolygonHermez/pil2-proofman.git || return 1
    cd pil2-proofman
    # If PIL2_PROOFMAN_BRANCH is defined, check out the specified branch
    if [[ -n "$PIL2_PROOFMAN_BRANCH" ]]; then
        echo "Checking out branch '$PIL2_PROOFMAN_BRANCH' for pil2-proofman..."
        ensure git checkout "$PIL2_PROOFMAN_BRANCH" || return 1
    fi
    cd ..

    ensure git clone https://github.com/0xPolygonHermez/pil2-proofman-js.git || return 1
    cd pil2-proofman-js
    # If PIL2_PROOFMAN_JS_BRANCH is defined, check out the specified branch
    if [[ -n "$PIL2_PROOFMAN_JS_BRANCH" ]]; then
        echo "Checking out branch '$PIL2_PROOFMAN_JS_BRANCH' for pil2-proofman-js..."
        ensure git checkout "$PIL2_PROOFMAN_JS_BRANCH" || return 1
    fi
    rm -rf package-lock.json
    rm -rf node_modules
    cd ..

    press_any_key

    step "Installing npm packages..."
    cd pil2-compiler
    ensure npm i || return 1
    cd ..

    cd pil2-proofman-js 
    ensure npm i || return 1
    cd ..

    cd "$(get_zisk_repo_dir)"

    step "Generate fixed data..."
    ensure cargo run --release --bin keccakf_fixed_gen || return 1

    step "Compiling ZisK PIL..."
    ensure node ../pil2-compiler/src/pil.js pil/zisk.pil \
	-I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines,precompiles \
	-o pil/zisk.pilout -u tmp/fixed -O fixed-to-file || return 1

    step "Generating setup..."
    cached=0
    if [[ "${USE_SETUP_CACHE}" == "1" ]]; then
        # Compute setup hash
        HASH_SUM=$(sha256sum pil/zisk.pilout tmp/fixed/*.fixed \
        | sort -k2 \
        | sha256sum \
        | awk '{print $1}' \
        | awk '{print substr($0, 1, 4) substr($0, length($0)-3)}')

        echo "Setup hash: ${HASH_SUM}"

        ZISK_VERSION=$(echo "$(ensure cargo-zisk --version)" | awk '{print $2}')
        IFS='.' read -r major minor patch <<< "${ZISK_VERSION}"
        cache_setup_folder="${OUTPUT_DIR}/${major}.${minor}.0/${HASH_SUM}"

        # Check if setup file exists in cache
        if [[ -d "${cache_setup_folder}" ]]; then
            info "Found cached setup folder: ${cache_setup_folder}"
            cached=1
        else
            info "No cached setup folder found at ${cache_setup_folder}"
        fi
    fi

    if [[ ${cached} == "0" ]]; then
        if [[ ${DISABLE_RECURSIVE_SETUP} == "1" ]];  then
            info "Building non-recursive setup..."
        else
            info  "Building recursive setup..."
            # Add flags for recursive setup command
            setup_flags="-t ../pil2-proofman/pil2-components/lib/std/pil -r"
            # Add -a flag  (aggregation) for check-setup command
            check_setup_flags=-a
        fi

        ensure node ../pil2-proofman-js/src/main_setup.js \
            -a ./pil/zisk.pilout -b build \
            -u tmp/fixed ${setup_flags}
    fi

    if [[ ${USE_SETUP_CACHE} == "1" && ${cached} == "0" ]]; then
        info "Caching setup..."
        mkdir -p "${cache_setup_folder}"
        ensure cp -R build/provingKey "${cache_setup_folder}" || return 1
    fi

    step "Copy provingKey directory to \$HOME/.zisk directory..."
    if [[ ${cached} == "1" ]]; then
        ensure cp -R "${cache_setup_folder}/provingKey" "$HOME/.zisk" || return 1
    else
        ensure cp -R build/provingKey "$HOME/.zisk" || return 1
    fi

    step "Generate constant tree files..."
    ensure cargo-zisk check-setup $check_setup_flags || return 1

    success "ZisK setup completed successfully!"
}

main
