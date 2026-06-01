#!/bin/bash

source "./utils.sh"
source "./deploy_distributed.sh"

main() {
    current_step=1
    total_steps=6

    step "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

    ensure cd "${WORKSPACE_DIR}" || return 1

    ZEC_RETH_ELF="${WORKSPACE_DIR}/zisk-eth-client/bin/guests/stateless-validator-reth/target/elf/riscv64ima-zisk-zkvm-elf/release/zec-reth"
    ZEC_RETH_INPUTS="${WORKSPACE_DIR}/zisk-eth-client/bin/guests/stateless-validator-reth/inputs"

    step "Verifying zec-reth ELF exists..."
    if [[ ! -f "${ZEC_RETH_ELF}" ]]; then
        err "zec-reth ELF not found: ${ZEC_RETH_ELF}. Please run build_zec_reth.sh first."
        return 1
    fi

    step "Cloning zisk-ethproofs repository..."
    if [[ -n "${DISABLE_CLONE_REPO:-}" && "$DISABLE_CLONE_REPO" == "1" ]]; then
        warn "Skipping cloning zisk-ethproofs repository as DISABLE_CLONE_REPO is set to 1"
    else
        # Remove existing directory if it exists
        rm -rf zisk-ethproofs
        # Clone zisk-ethproofs repository
        if [[ -n "${ZISK_ETHPROOFS_BRANCH:-}" ]]; then
            info "Cloning branch '$ZISK_ETHPROOFS_BRANCH' of zisk-ethproofs..."
            ensure git clone --branch "$ZISK_ETHPROOFS_BRANCH" --single-branch --depth 1 https://github.com/0xPolygonHermez/zisk-ethproofs.git || return 1
        else
            ensure git clone --depth 1 https://github.com/0xPolygonHermez/zisk-ethproofs.git || return 1
        fi
    fi

    step "Building zisk-ethproofs..."
    ensure cd zisk-ethproofs || return 1
    local cfg_hints=""
    [[ "${ENABLE_HINTS:-}" == "1" ]] && cfg_hints="RUSTFLAGS='--cfg zisk_hints --cfg zisk_hints_metrics --cfg zisk_hints_single_thread'"
    ensure eval "$cfg_hints cargo build --release" || return 1
    cd ..

    step "Deploying ZisK coordinator and worker services..."
    deploy_distributed

    step "Executing ethproofs-client tests..."
    ensure cd zisk-ethproofs || return 1
    local input_files_arg=""
    if [[ "${ENABLE_HINTS:-}" == "1" ]]; then
        [[ -n "${BLOCK_INPUTS_ETHPROOFS_HINTS:-}" ]] && input_files_arg="--folder.input-files ${BLOCK_INPUTS_ETHPROOFS_HINTS}"
    else
        [[ -n "${BLOCK_INPUTS_ETHPROOFS:-}" ]] && input_files_arg="--folder.input-files ${BLOCK_INPUTS_ETHPROOFS}"
    fi
    ensure ./target/release/ethproofs-client \
        -c http://localhost:7010 \
        --input.folder "${WORKSPACE_DIR}/zisk-ethproofs/inputs" \
        -n folder \
        -g "$ZEC_RETH_ELF" \
        --folder.path "$ZEC_RETH_INPUTS" \
        ${input_files_arg:+$input_files_arg} \
        --exit-on-error \
        || return 1
}

trap uninstall_distributed EXIT
main