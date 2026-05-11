source "./utils.sh"
source "./deploy_distributed.sh"

main() {
    current_step=1
    total_steps=6

    step "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

    ZISK_GHA=1
    export ZISK_GHA

    cd "${WORKSPACE_DIR}"

    step "Cloning zisk-eth-client repository..."
    if [[ -n "${DISABLE_CLONE_REPO:-}" && "$DISABLE_CLONE_REPO" == "1" ]]; then
        warn "Skipping cloning zisk-eth-client repository as DISABLE_CLONE_REPO is set to 1"
    else
        # Remove existing directory if it exists
        rm -rf zisk-eth-client
        # Clone zisk-eth-client repository
        if [[ -n "${ZISK_ETH_CLIENT_BRANCH:-}" ]]; then
            info "Cloning branch '$ZISK_ETH_CLIENT_BRANCH' of zisk-eth-client..."
            ensure git clone --branch "$ZISK_ETH_CLIENT_BRANCH" --single-branch --depth 1 https://github.com/0xPolygonHermez/zisk-eth-client.git || return 1
        else
            ensure git clone --depth 1 https://github.com/0xPolygonHermez/zisk-eth-client.git || return 1
        fi
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
    ensure cd zisk-ethproofs
    local cfg_hints=""
    [[ "${ENABLE_HINTS:-}" == "1" ]] && cfg_hints="RUSTFLAGS='--cfg zisk_hints --cfg zisk_hints_metrics --cfg zisk_hints_single_thread'"
    ensure eval "$cfg_hints cargo build --release" || return 1
    cd ..

    step "Deploying ZisK coordinator and worker services..."
    deploy_distributed

    step "Executing ethproofs-client tests..."
    #RPC_URL=http://144.76.59.84:8545
    #RPC_WS_URL=ws://144.76.59.84:8546
    BLOCK_MODULUS=1
    COORDINATOR_URL=http://localhost:7010
    INPUTS_FOLDER="${WORKSPACE_DIR}/zisk-ethproofs/inputs"
    COMPUTE_CAPACITY=10
    ensure export RPC_URL RPC_WS_URL BLOCK_MODULUS COORDINATOR_URL INPUTS_FOLDER COMPUTE_CAPACITY

    ensure cd zisk-ethproofs
    local input_files_arg=""
    if [[ "${ENABLE_HINTS:-}" == "1" ]]; then
        [[ -n "${BLOCK_INPUTS_ETHPROOFS_HINTS:-}" ]] && input_files_arg="--input-files ${BLOCK_INPUTS_ETHPROOFS_HINTS}"
    else
        [[ -n "${BLOCK_INPUTS_ETHPROOFS:-}" ]] && input_files_arg="--input-files ${BLOCK_INPUTS_ETHPROOFS}"
    fi
    ensure ./target/release/ethproofs-client \
        -n folder \
        -g ../zisk-eth-client/bin/guests/stateless-validator-reth/elf/zec-reth.elf \
        --inputs-queue ../zisk-eth-client/bin/guests/stateless-validator-reth/inputs \
        ${input_files_arg:+$input_files_arg} \
        --exit-on-error \
        || return 1
}

trap uninstall_distributed EXIT
main