#!/usr/bin/env bash
# Builds the full ZisK PIL, regenerates the Rust helpers and produces basic, recursive and
# PLONK keys in an isolated directory. Phases: preflight, compile, setup-basic,
# setup-recursive, kernels, snark. Every input is pinned by version or hash.
set -euo pipefail

task_phase=${1:?Usage: e2e-build.sh PHASE ISOLATED_TASK_DIR}
task_root=$(realpath -e -- "${2:?Set the isolated task directory}")
task_repo="$task_root/src/zisk"
task_proofman="$task_root/src/proofman"
task_artifacts="$task_root/artifacts"
task_registry=${PROOFMAN_REGISTRY_DIR:?Set the registry directory containing Proofman 1.2.0-alpha}
task_compiler="$task_registry/pil2-stark-setup-1.2.0-alpha/node_modules/pil2-compiler"
task_setup=${PROOFMAN_SETUP_BIN:?Set the exact version-matching cargo-zisk-dev binary}
task_bins=${ZISK_E2E_BIN_DIR:-$task_root/cargo-target/release}
task_pilout="$task_artifacts/zisk.pilout"
task_fixed="$task_artifacts/fixed"

test "$task_root" != / && test "$task_root" != /root
test "$task_root" != "${HOME:?}" && test "$task_root" != "${HOME}/.zisk"
test -f "$task_repo/Cargo.toml"
test -f "$task_proofman/Cargo.toml"
test -f "$task_compiler/src/pil.js"
test -x "$task_setup"
test -d "$task_artifacts"

node - "$task_registry/pil2-stark-setup-1.2.0-alpha" <<'JS'
const fs = require('node:fs');
const path = require('node:path');
const root = process.argv[2];
const vcs = JSON.parse(fs.readFileSync(path.join(root, '.cargo_vcs_info.json')));
const lock = JSON.parse(fs.readFileSync(path.join(root, 'package-lock.json')));
if (vcs.git.sha1 !== '9ca9a2f11075ab458360c07f164573b8aefddde1') {
  throw new Error('Proofman setup source revision does not match the pinned baseline');
}
const compiler = lock.packages['node_modules/pil2-compiler'];
if (!compiler.resolved.endsWith('#503862ca92a7493623bca1bc331cdbfb9398e435')) {
  throw new Error('PIL compiler lock does not match the pinned baseline');
}
JS

export ZISK_CACHE_DIR="$task_root/cache"
export CARGO_TARGET_DIR="$task_root/cargo-target"
export PIL2C_EXEC="$task_compiler/src/pil.js"
export NODE_OPTIONS=${NODE_OPTIONS:---max-old-space-size=8192}
export OMP_NUM_THREADS=${OMP_NUM_THREADS:-4}
export RAYON_NUM_THREADS=${RAYON_NUM_THREADS:-4}
unset ZISK_USE_INSTALLED

task_circom="$task_proofman/setup/stark-recurser/stark2circom/circom_verifier"
export CIRCOM_HELPERS_DIR="$task_proofman/setup/circom"
export FINAL_SNARK_CIRCOM_HELPERS_DIR="$task_proofman/setup/final_snark_circom"
export CIRCUITS_GL_PATH="$task_circom/circuits.gl"
export CIRCUITS_BN128_PATH="$task_circom/circuits.bn128"
export RECURSER_CIRCUITS_PATH="$task_circom/helper_circuits"
export RECURSER_CIRCUITS_COMPRESSED_FINAL_PATH="$task_circom/helper_circuits"
export RECURSER_PIL_PATH="$task_proofman/setup/stark-recurser/plonk2pil/pil"
export STD_PIL_PATH="$task_proofman/pil2-components/lib/std/pil"
export GOLDILOCKS_SRC_DIR="$task_proofman/pil2-stark/src/goldilocks/src"

cd -- "$task_repo"
case "$task_phase" in
    preflight)
        "$task_setup" --version
        node --version
        sha256sum "$task_repo/Cargo.lock" "$task_compiler/src/pil.js" \
            "$task_registry/proofman-starks-src-1.2.0-alpha/lib-gpu/libstarksgpu.a"
        df -h -- "$task_root"
        ;;
    compile)
        task_helpers=${PROOFMAN_CLI_BIN:?Set the exact version-matching proofman-cli binary}
        test -x "$task_helpers"
        task_generators=(zisk-arith-frops-fixed-gen zisk-binary-basic-frops-fixed-gen
            zisk-binary-extension-frops-fixed-gen jump-dest-bitmap-table-gen)
        for task_generator in "${task_generators[@]}"; do
            test -x "$task_bins/$task_generator"
        done
        test ! -e "$task_pilout"
        mkdir -p -- "$task_fixed"
        for task_generator in "${task_generators[@]}"; do
            "$task_bins/$task_generator"
        done
        node "$PIL2C_EXEC" pil/zisk.pil \
            -I "pil,$STD_PIL_PATH,state-machines,precompiles" \
            -o "$task_pilout" -f "$task_fixed" \
            -O fixed-to-file -O no-proto-fixed-data
        "$task_helpers" pil-helpers --pilout "$task_pilout" --path pil/src -o
        sha256sum "$task_pilout" pil/src/pil_helpers/traces.rs
        ;;
    setup-basic|setup-recursive)
        test -f "$task_pilout"
        test -d "$task_fixed"
        task_kind=${task_phase#setup-}
        task_build="$task_root/setup/$task_kind"
        test ! -e "$task_build/provingKey"
        task_recursive=()
        if [ "$task_kind" = recursive ]; then
            task_recursive=(--recursive --recursive-jobs 1)
        fi
        "$task_setup" proofman-setup setup --airout "$task_pilout" \
            --build-dir "$task_build" --fixed-dir "$task_fixed" \
            --stark-structs state-machines/starkstructs.json \
            --hash Poseidon1 --setup-jobs 1 "${task_recursive[@]}" \
            --output "$task_artifacts/$task_kind-setup-stats.txt"
        ;;
    kernels)
        task_cuda_arch=${ZISK_E2E_CUDA_ARCH:?Set explicit CUDA architectures, e.g. 89 or sm_120}
        if [[ ! "$task_cuda_arch" =~ ^(sm_)?[1-9][0-9]{1,2}(,(sm_)?[1-9][0-9]{1,2})*$ ]]; then
            printf 'Invalid ZISK_E2E_CUDA_ARCH: use explicit architectures such as 89, sm_120 or 89,120.\n' >&2
            exit 2
        fi
        test -d "$task_root/setup/recursive/provingKey"
        command -v nvcc >/dev/null
        "$task_setup" proofman-setup gen-exps \
            --proving-key "$task_root/setup/recursive/provingKey" \
            --arch "$task_cuda_arch" --stark-src "$task_proofman/pil2-stark"
        ;;
    snark)
        task_tau=${ZISK_E2E_PTAU:?Set the existing trusted powers-of-tau file}
        test -f "$task_tau"
        test -d "$task_root/setup/recursive/provingKey"
        test ! -e "$task_root/setup/recursive/provingKeySnark"
        "$task_setup" proofman-setup setup-snark \
            --build-dir "$task_root/setup/recursive" \
            --publics-info state-machines/publics.json \
            --powers-of-tau "$task_tau" --final-snark plonk
        ;;
    *)
        printf 'Unknown phase: %s\n' "$task_phase" >&2
        exit 2
        ;;
esac
