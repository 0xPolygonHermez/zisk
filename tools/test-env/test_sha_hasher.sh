#!/bin/bash

source "./utils.sh"

PROJECT_NAME="sha_hasher"
EXPECTED_OUTPUT="98211882|bd13089b|6ccf1fca|81f7f0e4|abf6352a|0c39c9b1|1f142cac|233f1280"

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    current_step=1
    if [[ "${DISABLE_PROVE}" == "1" ]]; then
        total_steps=8
    else
        total_steps=10
    fi

    if [[ "${PLATFORM}" == "linux" ]]; then
        is_proving_key_installed || return 1
    fi   

    step "Loading environment variables..."
    # Load environment variables from .env file
    load_env || return 1
    confirm_continue || return 0

    cd "${WORKSPACE_DIR}"

    step "Deleting shared memory..."
    rm -rf /dev/shm/ZISK*
    rm -rf /dev/shm/sem*

    step "Creating new ZisK program: $PROJECT_NAME"
    rm -rf "$PROJECT_NAME"
    ensure cargo-zisk sdk new "$PROJECT_NAME" || return 1
    cd "$PROJECT_NAME"

    step "Building program..."
    ensure cargo-zisk build --release || return 1

    ELF_PATH="target/riscv64ima-zisk-zkvm-elf/release/$PROJECT_NAME"
    INPUT_BIN="build/input.bin"

    step "Running program with ziskemu..."
    ensure ziskemu -e "$ELF_PATH" -i "$INPUT_BIN" | tee ziskemu_output.log || return 1
    if ! grep -qE ${EXPECTED_OUTPUT} ziskemu_output.log; then
        err "run ziskemu failed"
        return 1
    fi

    step "Running program with cargo-zisk run..."
    ensure cargo-zisk run --release -i build/input.bin | tee run_output.log || return 1
    if ! grep -qE ${EXPECTED_OUTPUT} run_output.log; then
        err "run program failed"
        return 1
    fi

    if [[ "${PLATFORM}" == "linux" ]]; then
        step "Generating program setup..."
        ensure cargo-zisk rom-setup -e "$ELF_PATH" 2>&1 | tee romsetup_output.log || return 1
        if ! grep -F "ROM setup successfully completed" romsetup_output.log; then
            err "program setup failed"
            return 1
        fi

        step "Verifying constraints..."
        if [[ "${BUILD_GPU}" == "1" ]]; then
            warn "Skipping verify constraints step for GPU mode (not supported yet)"
        else    
            ensure cargo-zisk verify-constraints -e "$ELF_PATH" -i "$INPUT_BIN" 2>&1 | tee constraints_output.log || return 1
            if ! grep -F "All global constraints were successfully verified" constraints_output.log; then
                err "verify constraints failed"
                return 1
            fi
        fi

        if [[ "${DISABLE_PROVE}" != "1" ]]; then
            step "Generating proof..."
            MPI_CMD=""
            # If ZISK_GHA is set, use mpirun command for distributed proving to prove it faster and reduce GHA time
            if is_gha; then
                # Build mpi command
                info "Using mpirun for distributed proving"
                MPI_CMD="mpirun --allow-run-as-root --bind-to none -np $DISTRIBUTED_PROCESSES -x OMP_NUM_THREADS=$DISTRIBUTED_THREADS -x RAYON_NUM_THREADS=$DISTRIBUTED_THREADS"
            fi
            ensure $MPI_CMD cargo-zisk prove -e "$ELF_PATH" -i "$INPUT_BIN" -o proof $PROVE_FLAGS 2>&1 | tee prove_output.log || return 1
            if ! grep -F "Vadcop Final proof was verified" prove_output.log; then
                err "prove program failed"
                return 1
            fi

            step "Verifying proof..."
            ensure cargo-zisk verify -p ./proof/vadcop_final_proof.bin 2>&1 | tee verify_output.log || return 1
            if ! grep -F "Stark proof was verified" verify_output.log; then
                err "verify proof failed"
                return 1
            fi
        fi
    fi

    cd "$current_dir"

    success "Program $PROJECT_NAME has been successfully proved!"
}

main
