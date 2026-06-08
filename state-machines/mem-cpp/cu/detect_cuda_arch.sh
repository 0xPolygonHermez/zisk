#!/bin/bash
# Detect the host GPU compute capability and write CUDA_ARCH = sm_XXX to CudaArch.mk.
# Used by Makefile when neither CUDA_GENCODE_FLAGS nor CUDA_ARCH are set externally.
#
# Detection order:
#   1. nvidia-smi --query-gpu=compute_cap (modern, no CUDA samples needed)
#   2. Existing CudaArch.mk left in place (cached previous detection)
#   3. Error out — user must set CUDA_ARCHS / CUDA_ARCH / CUDA_GENCODE_FLAGS
#

set -eu

OUT_FILE="CudaArch.mk"

if command -v nvidia-smi >/dev/null 2>&1; then
    # compute_cap output is e.g. "8.0" / "12.0" — strip the dot to get sm_XXX.
    CAP=$(nvidia-smi --query-gpu=compute_cap --format=csv,noheader 2>/dev/null \
        | head -n1 \
        | tr -d ' .')
    if [ -n "$CAP" ]; then
        echo "CUDA_ARCH = sm_${CAP}" > "$OUT_FILE"
        echo "[detect_cuda_arch] Host GPU compute capability ${CAP} → CUDA_ARCH = sm_${CAP}"
        exit 0
    fi
fi

# Fallback: keep an existing CudaArch.mk if present (previous successful detection)
if [ -f "$OUT_FILE" ]; then
    echo "[detect_cuda_arch] nvidia-smi unavailable; reusing cached $OUT_FILE"
    exit 0
fi

echo "[detect_cuda_arch] ERROR: nvidia-smi not found and no cached CudaArch.mk." >&2
echo "    Set CUDA_ARCHS (e.g. CUDA_ARCHS=\"major\" or CUDA_ARCHS=\"89\")," >&2
echo "    CUDA_ARCH (e.g. CUDA_ARCH=sm_89), or CUDA_GENCODE_FLAGS explicitly." >&2
exit 1
