#!/bin/bash

source ./utils.sh

main() {
    current_step=1
    total_steps=9

    step "Loading environment variables..."
    load_env || return 1
    confirm_continue || return 1

    mkdir -p "${HOME}/work"
    cd "${HOME}/work"

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

    step  "Installing npm packages..."
    cd pil2-compiler
    ensure npm i || return 1
    cd ..

    cd pil2-proofman-js 
    ensure npm i || return 1
    cd ..

    cd zisk

    step  "Preparing environment for setup..."
    echo "vm.max_map_count=655300" >> /etc/sysctl.conf
    sysctl -w vm.max_map_count=655300
    export NODE_OPTIONS="--max-old-space-size=230000"

    step  "Compiling ZisK PIL..."
    ensure node --max-old-space-size=131072 ../pil2-compiler/src/pil.js pil/zisk.pil \
        -I pil,../pil2-proofman/pil2-components/lib/std/pil,state-machines,precompiles -o pil/zisk.pilout || return 1

    step  "Generate fixed data..."
    ensure cargo run --release --bin keccakf_fixed_gen || return 1

    step  "Generate setup data..."
    ensure node --max-old-space-size=131072 --stack-size=1500 ../pil2-proofman-js/src/main_setup.js \
        -a ./pil/zisk.pilout -b build \
        -i precompiles/keccakf/src/keccakf_fixed.bin -r || return 1

    step "Copy provingKey directory to \$HOME/.zisk directory..."
    ensure cp -R build/provingKey "$HOME/.zisk" || return 1

    step "Generate constant tree files..."
    ensure cargo-zisk check-setup -a || return 1

    cd ..
}

main || return 1