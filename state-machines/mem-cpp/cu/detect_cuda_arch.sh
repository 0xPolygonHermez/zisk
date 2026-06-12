#!/bin/bash
# Detect the host GPU compute capability and write CUDA_ARCH = sm_XXX to CudaArch.mk.
# Used by Makefile when neither CUDA_GENCODE_FLAGS, CUDA_ARCHS nor CUDA_ARCH are
# set externally.
#
# On failure exits 1
set -eu

OUT_FILE="CudaArch.mk"
rm -f "$OUT_FILE"

if command -v nvidia-smi >/dev/null 2>&1; then
    # compute_cap output is e.g. "8.0" / "12.0" — strip the dot to get sm_XXX.
    CAP=$(nvidia-smi --query-gpu=compute_cap --format=csv,noheader 2>/dev/null \
        | head -n1 \
        | tr -d ' .')
    case "$CAP" in
        ''|*[!0-9]*) ;; # empty or non-numeric (e.g. driver up but no GPU) — ignore
        *)
            echo "CUDA_ARCH = sm_${CAP}" > "$OUT_FILE"
            echo "[detect_cuda_arch] Host GPU compute capability ${CAP} → CUDA_ARCH = sm_${CAP}"
            exit 0
            ;;
    esac
fi

echo "[detect_cuda_arch] nvidia-smi probe failed — no GPU arch detected, falling back to major archs." >&2
exit 1
