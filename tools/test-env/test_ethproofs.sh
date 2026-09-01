#!/bin/bash

source "./utils.sh"
source "./deploy_distributed.sh"

main() {
    current_step=1
    total_steps=4

    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    info "Loading environment variables..."
    # Load environment variables from .env file (only the ones used by this script)
    load_env ENABLE_HINTS BLOCK_INPUTS_ETHPROOFS BLOCK_INPUTS_ETHPROOFS_HINTS ONLY_CPU ZEC_GUEST || return 1

    # Exports ENABLE_HINTS too, so the build_ethproofs.sh run below agrees.
    resolve_zec_guest || return 1

    ensure cd "${WORKSPACE_DIR}" || return 1

    step "Verifying zec-${ZEC_GUEST} ELF exists..."
    if [[ ! -f "${ZEC_ELF}" ]]; then
        err "zec-${ZEC_GUEST} ELF not found: ${ZEC_ELF}. Please run build_zec_guest.sh first."
        return 1
    fi

    step "Building ethproofs-client..."
    ensure bash -c "cd \"${script_dir}\" && ./build_ethproofs.sh" || return 1
    ensure cd "${WORKSPACE_DIR}" || return 1

    step "Deploying ZisK coordinator and worker services..."
    deploy_distributed || return 1

    step "Executing ethproofs-client tests..."
    ensure cd zisk-ethproofs || return 1
    local input_files_arg=""
    if [[ "${ENABLE_HINTS:-}" == "1" ]]; then
        zec_guest_inputs BLOCK_INPUTS_ETHPROOFS_HINTS || return 1
        [[ -n "${BLOCK_INPUTS_ETHPROOFS_HINTS:-}" ]] && input_files_arg="--folder.input-files ${BLOCK_INPUTS_ETHPROOFS_HINTS}"
    else
        zec_guest_inputs BLOCK_INPUTS_ETHPROOFS || return 1
        [[ -n "${BLOCK_INPUTS_ETHPROOFS:-}" ]] && input_files_arg="--folder.input-files ${BLOCK_INPUTS_ETHPROOFS}"
    fi
    ensure ./target/release/ethproofs-client \
        -c http://localhost:7010 \
        --input.folder "${WORKSPACE_DIR}/zisk-ethproofs/inputs" \
        -n folder \
        -g "$ZEC_ELF" \
        --folder.path "$ZEC_INPUTS" \
        ${input_files_arg:+$input_files_arg} \
        --exit-on-error \
        || return 1
}

trap uninstall_distributed EXIT
main
