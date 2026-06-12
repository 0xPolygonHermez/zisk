#!/bin/bash

source "./utils.sh"

# group_start: announce a test group with a wall-clock timestamp.
group_start() {
    echo "
${BOLD}${GREEN}[$(date '+%Y-%m-%d %H:%M:%S')] ${1} ${RESET}
"
}

# cmd_step: announce one command (counts as a single step). Prints the running
# step counter followed by the icon-styled command. The caller runs the command.
#   $1 (icon_label) – short label shown after the ▶ icon (e.g. "Build")
#   $2 (command)    – the full command line being run
cmd_step() {
    echo -e "   ${BOLD}${YELLOW}[${current_step}]${RESET} ${YELLOW}▶ ${1}:${RESET} ${2}"
    current_step=$(( current_step + 1 ))
}

# build_flag_combos: Populate the global FLAG_COMBOS array with every
# combination (power set) of the available {asm, gpu} capabilities, always
# including the empty "no flags" combination. Each element is a
# space-separated token list, e.g. "" / "asm" / "gpu" / "asm gpu".
#
# So with both asm and gpu available you get 4 combos; with only one you get
# that one plus no-flags; with neither, just no-flags.
#
# When GPU_ONLY=1, every combination that wouldn't engage the GPU is dropped, so
# the run never exercises a non-GPU path — and it errors out if no GPU backend
# is available.
#
# Reads has_asm / has_gpu / ONLY_CPU / GPU_ONLY from the calling scope.
build_flag_combos() {
    local gpu_available=0
    [[ "${has_gpu:-0}" -eq 1 ]] && [[ "${ONLY_CPU:-}" != "1" ]] && gpu_available=1

    if [[ "${GPU_ONLY:-}" == "1" ]] && [[ "${gpu_available}" -ne 1 ]]; then
        err "--gpu-only was requested but no GPU backend is available"
        return 1
    fi

    local -a caps=()
    [[ "${has_asm:-0}" -eq 1 ]] && caps+=("asm")
    [[ "${gpu_available}" -eq 1 ]] && caps+=("gpu")

    FLAG_COMBOS=("")
    local cap existing
    for cap in "${caps[@]}"; do
        local -a extended=()
        for existing in "${FLAG_COMBOS[@]}"; do
            extended+=("${existing:+${existing} }${cap}")
        done
        FLAG_COMBOS+=("${extended[@]}")
    done

    # In GPU-only mode, keep only the combinations that engage the GPU.
    if [[ "${GPU_ONLY:-}" == "1" ]]; then
        local -a gpu_combos=()
        for existing in "${FLAG_COMBOS[@]}"; do
            [[ " ${existing} " == *" gpu "* ]] && gpu_combos+=("${existing}")
        done
        FLAG_COMBOS=("${gpu_combos[@]}")
    fi
}

# run_sdk_example: Build and run a ZisK example via the host, verifying success.
#
# Runs once per available flag combination (power set of asm/gpu). The host
# binaries parse `--asm`/`--gpu` from their own argv (via examples_utils::
# parse_args), so the tokens are passed as runtime args after `--`, not as
# cargo --features. See build_flag_combos.
#
# Parameters:
#   $1 (name)          – Example directory name under examples/
#   $2 (bin_name)      – Binary name for --bin flag (empty = default binary)
#   $3 (success_str)   – Fixed string to grep in output to confirm success
#                        (empty = rely on exit code only)
#   $4 (require_asm)   – Set to 1 if the example only works with the asm backend
#                        (e.g. primes/streaming); non-asm combos are then skipped.
#   $5 (flag_agnostic) – Set to 1 for examples that ignore the asm/gpu flags
#                        (e.g. the merkle-tree profiler); runs only once.
run_example() {
    local name="$1"
    local bin_name="${2:-}"
    local success_str="${3:-}"
    local require_asm="${4:-0}"
    local flag_agnostic="${5:-0}"
    local base_label="${name}${bin_name:+/${bin_name}}"

    local host_dir="${EXAMPLES_DIR}/${name}/host"
    if [[ ! -d "${host_dir}" ]]; then
        err "Example host directory not found: ${host_dir}"
        return 1
    fi

    local bin_flag=""
    [[ -n "${bin_name}" ]] && bin_flag="--bin ${bin_name}"

    group_start "Host example: ${base_label}"

    # Flag-agnostic examples don't read asm/gpu, so a single run is enough.
    local -a combos=("${FLAG_COMBOS[@]}")
    [[ "${flag_agnostic}" -eq 1 ]] && combos=("")

    local combo
    for combo in "${combos[@]}"; do
        local combo_label="${combo:-no-flags}"
        local label="${base_label} [${combo_label}]"
        local log_file="${LOG_DIR}/${base_label//\//_}_${combo_label// /_}.log"

        # Some examples (e.g. primes/streaming) only run on the asm backend.
        if [[ "${require_asm}" -eq 1 ]] && [[ " ${combo} " != *" asm "* ]]; then
            echo -e "   ${YELLOW}▶ Executing [${combo_label}]:${RESET} skipped (${base_label} requires the asm backend)"
            continue
        fi

        # Map the capability tokens to runtime flags consumed by the host binary.
        local run_flags=""
        [[ " ${combo} " == *" asm "* ]] && run_flags+=" --asm"
        [[ " ${combo} " == *" gpu "* ]] && run_flags+=" --gpu"

        pushd "${host_dir}" > /dev/null

        cmd_step "Executing [${combo_label}]" "cargo run --release ${bin_flag} --${run_flags}"
        # shellcheck disable=SC2086
        ensure cargo run --release ${bin_flag} -- ${run_flags} > "${log_file}" 2>&1
        local exit_code=$?

        popd > /dev/null

        if [[ ${exit_code} -ne 0 ]]; then
            cat "${log_file}"
            err "Example ${label} failed (exit code ${exit_code})"
            return 1
        fi

        if [[ -n "${success_str}" ]]; then
            if ! grep -qF "${success_str}" "${log_file}"; then
                cat "${log_file}"
                err "Example ${label}: expected '${success_str}' not found in output"
                return 1
            fi
        fi
    done

    return 0
}

# run_guest_example: Build, run, execute, and prove a ZisK guest program.
#
# Parameters:
#   $1 (name)         – Example directory name under examples/
#   $2 (bin_name)     – Binary name for --bin flag (empty = default binary)
#   $3 (input_file)   – Input file path relative to guest dir (empty = no input)
#   $4 (success_str)  – Fixed string to grep in output to confirm success
#                       (empty = rely on exit code only)
#   $5 (skip_execute) – Set to 1 to skip the execute step (e.g. profiling
#                       variants that aren't meant for the prover path)
#   $6 (skip_prove)   – Set to 1 to skip the prove step (same rationale)
#
# Steps: build → run (once) → execute + prove (once per flag combo, with
# inline verification). asm affects execute and prove; gpu affects prove only.
# Reads FLAG_COMBOS from the calling scope (set via build_flag_combos in main).
run_guest_example() {
    local name="$1"
    local bin_name="${2:-}"
    local input_file="${3:-}"
    local success_str="${4:-}"
    local skip_execute="${5:-0}"
    local skip_prove="${6:-0}"
    local label="${name}${bin_name:+/${bin_name}}"
    local log_file="${LOG_DIR}/guest_${label//\//_}.log"

    local guest_dir="${EXAMPLES_DIR}/${name}/guest"
    if [[ ! -d "${guest_dir}" ]]; then
        err "Example guest directory not found: ${guest_dir}"
        return 1
    fi

    local bin_flag=""
    [[ -n "${bin_name}" ]] && bin_flag="--bin ${bin_name}"

    local input_flag=""
    [[ -n "${input_file}" ]] && input_flag="-i ${input_file}"

    group_start "Guest example: ${label}"

    pushd "${guest_dir}" > /dev/null

    # 1. Build (independent of asm/gpu, so done once). Each command is one step.
    cmd_step "Build" "cargo-zisk build --release ${bin_flag}"
    # shellcheck disable=SC2086
    ensure cargo-zisk build --release ${bin_flag} > "${log_file}" 2>&1
    local exit_code=$?
    if [[ ${exit_code} -ne 0 ]]; then
        cat "${log_file}"
        popd > /dev/null
        err "Guest ${label}: build failed (exit code ${exit_code})"
        return 1
    fi

    # 2. Run (emulation, independent of asm/gpu, so done once)
    cmd_step "Run" "cargo-zisk run --release ${bin_flag} ${input_flag}"
    # shellcheck disable=SC2086
    ensure  cargo-zisk run --release ${bin_flag} ${input_flag} >> "${log_file}" 2>&1
    exit_code=$?
    if [[ ${exit_code} -ne 0 ]]; then
        cat "${log_file}"
        popd > /dev/null
        err "Guest ${label}: run failed (exit code ${exit_code})"
        return 1
    fi

    # 3 + 4. Execute + prove, once per available flag combination. When both
    # are skipped, the flag combos are irrelevant — report once and skip the loop.
    if [[ "${skip_execute}" -eq 1 ]] && [[ "${skip_prove}" -eq 1 ]]; then
        echo -e "   ${YELLOW}▶ Execute/Prove:${RESET} skipped for ${label}"
    else
        local combo
        for combo in "${FLAG_COMBOS[@]}"; do
            local combo_label="${combo:-no-flags}"

            # asm applies to execute and prove; gpu applies to prove only.
            local asm_flag="" gpu_flag=""
            [[ " ${combo} " == *" asm "* ]] && asm_flag="--asm"
            [[ " ${combo} " == *" gpu "* ]] && gpu_flag="--gpu"

            # 3. Execute (ZisK emulator)
            if [[ "${skip_execute}" -eq 1 ]]; then
                echo -e "   ${YELLOW}▶ Execute [${combo_label}]:${RESET} skipped for ${label}"
            else
                cmd_step "Execute [${combo_label}]" "cargo-zisk execute --release ${bin_flag} ${input_flag} ${asm_flag}"
                # shellcheck disable=SC2086
                ensure cargo-zisk execute --release ${bin_flag} ${input_flag} ${asm_flag} >> "${log_file}" 2>&1
                exit_code=$?
                if [[ ${exit_code} -ne 0 ]]; then
                    cat "${log_file}"
                    popd > /dev/null
                    err "Guest ${label} [${combo_label}]: execute failed (exit code ${exit_code})"
                    return 1
                fi
            fi

            # 4. Prove + verify inline
            if [[ "${skip_prove}" -eq 1 ]]; then
                echo -e "   ${YELLOW}▶ Prove [${combo_label}]:${RESET} skipped for ${label}"
            else
                cmd_step "Prove [${combo_label}]" "cargo-zisk prove --release ${bin_flag} ${input_flag} ${asm_flag} ${gpu_flag} --verify-proof"
                # shellcheck disable=SC2086
                ensure cargo-zisk prove --release ${bin_flag} ${input_flag} ${asm_flag} ${gpu_flag} --verify-proof >> "${log_file}" 2>&1
                exit_code=$?
                if [[ ${exit_code} -ne 0 ]]; then
                    cat "${log_file}"
                    popd > /dev/null
                    err "Guest ${label} [${combo_label}]: prove failed (exit code ${exit_code})"
                    return 1
                fi
            fi
        done
    fi

    popd > /dev/null

    if [[ -n "${success_str}" ]]; then
        if ! grep -qF "${success_str}" "${log_file}"; then
            cat "${log_file}"
            err "Guest ${label}: expected '${success_str}' not found in output"
            return 1
        fi
    fi

    return 0
}

main() {
    info "▶️  Running $(basename "$0") script..."

    current_dir=$(pwd)

    # Parse arguments. --gpu-only restricts the run to GPU flag combinations
    # (errors out below if no GPU backend is available).
    local arg
    for arg in "$@"; do
        case "${arg}" in
            --gpu-only) GPU_ONLY=1 ;;
            *) err "Unknown argument: ${arg}"; return 1 ;;
        esac
    done

    # primes/streaming requires the ASM backend (Linux only).
    local has_asm=0
    [[ "${PLATFORM}" != "darwin" ]] && has_asm=1

    local has_gpu=0
    cargo-zisk --version 2>/dev/null | grep -q "\[gpu\]" && has_gpu=1

    # Step counter advances per command; the total isn't known up front (it
    # depends on the active flag combinations), so step() shows a running count.
    current_step=1

    if ! is_gha || [[ "${PLATFORM}" == "linux" ]]; then
        is_proving_key_installed || return 1
    fi

    info "Loading environment variables..."
    load_env || return 1

    # Determine which flag combinations to test (power set of available asm/gpu).
    build_flag_combos || return 1
    local combos_desc="" c
    for c in "${FLAG_COMBOS[@]}"; do combos_desc+="[${c:-no-flags}] "; done
    info "Testing flag combinations: ${combos_desc}"

    rm -rf /dev/shm/ZISK*
    rm -rf /dev/shm/sem*+

    info "Deleting shared memory..."
    EXAMPLES_DIR="$(get_zisk_repo_dir)/examples"
    LOG_DIR="${WORKSPACE_DIR}/examples-logs"
    
    rm -rf "${LOG_DIR}"
    mkdir -p "${LOG_DIR}"

    # --- Guest examples (cargo-zisk build → run → execute → prove) ---
    
    run_guest_example "fibonacci"   "" "samples/example-input.bin" "verified successfully."   || return 1
    run_guest_example "hash"        "" "samples/example-input.bin" "verified successfully."   || return 1
    run_guest_example "gcd"         "" "samples/example-input.bin" "verified successfully."   || return 1

    # merkle-tree: two guest binaries sharing the same sample input. The
    # inline-profiling variant is build/run only — it generates no proof, so it
    # checks the run output instead of a verification line; the zisklib variant
    # is fully executed and proved.

    run_guest_example "merkle-tree" "inline-guest"  "samples/example-input.bin" "merkle-root(" 1 1 || return 1
    run_guest_example "merkle-tree" "zisklib-guest" "samples/example-input.bin" "verified successfully." || return 1

    # collatz: three guest binaries sharing the same sample input

    run_guest_example "collatz" "single-guest"     "samples/example-input.bin" "verified successfully." || return 1
    run_guest_example "collatz" "sequential-guest" "samples/example-input.bin" "verified successfully." || return 1
    run_guest_example "collatz" "compressed-guest" "samples/example-input.bin" "verified successfully." || return 1

    # primes: three guest binaries with per-variant sample inputs

    run_guest_example "primes" "struct-guest"   "samples/example-input-struct.bin"   "verified successfully." || return 1
    run_guest_example "primes" "multiple-guest" "samples/example-input-multiple.bin" "verified successfully." || return 1
    run_guest_example "primes" "slice-guest"    "samples/example-input-slice.bin"    "verified successfully." || return 1
    

    # --- Host examples ---

    run_example "fibonacci"   "" "verified successfully."  || return 1
    run_example "hash"        "" "verified successfully."  || return 1
    # merkle-tree host only profiles (no proof, ignores asm/gpu), so run it once
    # and rely on the exit code.
    run_example "merkle-tree" "" "" 0 1                     || return 1

    # gcd: prover-client and proof-format variants. The `remote` binary is
    # skipped because it targets a placeholder URL (https://prover.example.com).
    run_example "gcd" "embedded-host" "verified successfully." || return 1
    run_example "gcd" "stark-host"    "verified successfully." || return 1
    run_example "gcd" "minimal-host"  "verified successfully." || return 1
    # run_example "gcd" "plonk-host"    "verified successfully." || return 1

    # collatz (3 binaries)
    run_example "collatz" "single-host"     "verified successfully." || return 1
    run_example "collatz" "sequential-host" "verified successfully." || return 1
    run_example "collatz" "compressed-host" "verified successfully." || return 1

    # primes (3 always + streaming on Linux/ASM)
    run_example "primes" "struct-host"   "verified successfully." || return 1
    run_example "primes" "multiple-host" "verified successfully." || return 1
    run_example "primes" "slice-host"    "verified successfully." || return 1
    # streaming only works with the asm backend: require_asm=1 limits it to the
    # asm combos (and skips the example entirely when asm is unavailable).
    if [[ ${has_asm} -eq 1 ]]; then
        run_example "primes" "streaming-host" "verified successfully." 1 || return 1
    else
        warn "Skipping primes/streaming — ASM backend not supported on this platform"
    fi

    cd "$current_dir"

}

main "$@"

