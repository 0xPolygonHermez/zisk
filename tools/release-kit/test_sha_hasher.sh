#!/bin/bash

source "./utils.sh"

PROJECT_NAME="sha_hasher"
EXPECTED_OUTPUT="98211882|bd13089b|6ccf1fca|81f7f0e4|abf6352a|0c39c9b1|1f142cac|233f1280"

main() {
    current_step=1
    total_steps=10

    is_proving_key_installed || return 1
    
    step "Loading environment variables..."    
    load_env || return 1
    confirm_continue || return 1

    mkdir -p "${HOME}/work"
    cd "${HOME}/work" 

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

    step "Generating proof..."  
    ensure cargo-zisk prove -e "$ELF_PATH" -i "$INPUT_BIN" -o proof $PROVE_FLAGS 2>&1 | tee prove_output.log || return 1
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

    cd ..

    success "Program $PROJECT_NAME has been successfully proved!"
}

main || return 1
