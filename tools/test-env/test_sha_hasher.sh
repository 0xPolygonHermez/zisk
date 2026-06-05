#!/bin/bash

source "./utils.sh"

PROJECT_NAME="guest"
EXPECTED_OUTPUT="4fcbc136|2ce46a82|2248a8eb|785f0c7e|9dca7861|7267cace|d028d7e5|f6a2309c|000003e8|deadbeef"

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    if [[ "${DISABLE_PROVE}" == "1" ]]; then
        total_steps=8
    else
        total_steps=10
    fi
    if is_gha && [[ "${PLATFORM}" == "darwin" ]]; then
        total_steps=$((total_steps - 2))
    fi

    if ! is_gha || [[ "${PLATFORM}" == "linux" ]]; then
        is_proving_key_installed || return 1
    fi

    step "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1

    cd "${WORKSPACE_DIR}"

    step "Deleting shared memory..."
    rm -rf /dev/shm/ZISK*
    rm -rf /dev/shm/sem*

    step "Creating new ZisK program: $PROJECT_NAME"
    rm -rf "$PROJECT_NAME"
    ensure cargo-zisk new "$PROJECT_NAME" || return 1
    cd "$PROJECT_NAME"

    step "Building program..."
    ensure cargo build --bin host --release || return 1

    ELF_PATH="target/elf/riscv64ima-zisk-zkvm-elf/release/$PROJECT_NAME"
    INPUT_BIN="host/tmp/input.bin"

    step "Running program with ziskemu..."
    ensure ziskemu -e "$ELF_PATH" -i "$INPUT_BIN" -c | tee ziskemu_output.log || return 1
    if ! grep -qE ${EXPECTED_OUTPUT} ziskemu_output.log; then
        err "run ziskemu failed"
        return 1
    fi

    if is_gha && [[ "${PLATFORM}" == "darwin" ]]; then
        warn "Skipping prove and verify steps on macOS as it's not supported in GHA"
    else
        step "Generating program setup..."
        local gpu_flag=""
        [[ "${ONLY_CPU:-}" != "1" ]] && [[ "${PLATFORM}" != "darwin" ]] && gpu_flag="--gpu"
        ensure cargo-zisk-dev program-setup -e "$ELF_PATH" ${gpu_flag} 2>&1 | tee romsetup_output.log || return 1
        if ! grep -F "ROM setup successfully completed" romsetup_output.log; then
           err "program setup failed"
           return 1
        fi

        step "Verifying constraints..."
        ensure cargo-zisk-dev verify-constraints -e "$ELF_PATH" -i "$INPUT_BIN" ${gpu_flag} 2>&1 | tee constraints_output.log || return 1
        if ! grep -F "All global constraints were successfully verified" constraints_output.log; then
            err "verify constraints failed"
            return 1
        fi

        if [[ "${DISABLE_PROVE}" != "1" ]]; then
            step "Generating proof..."
            ensure cargo-zisk-dev prove -e "$ELF_PATH" -i "$INPUT_BIN" -o proof.bin $PROVE_FLAGS ${gpu_flag} 2>&1 | tee prove_output.log || return 1
            if ! grep -F "Vadcop Final proof was verified" prove_output.log; then
                err "prove program failed"
                return 1
            fi

            step "Verifying proof..."
            ensure cargo-zisk verify -p ./proof.bin 2>&1 | tee verify_output.log || return 1
            if ! grep -F "STARK proof was verified" verify_output.log; then
                err "verify proof failed"
                return 1
            fi
        fi
    fi

    cd "$current_dir"

    success "Program $PROJECT_NAME has been successfully proved!"
}

main
