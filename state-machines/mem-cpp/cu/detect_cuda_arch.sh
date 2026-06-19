#!/bin/bash
# Detect the host GPU compute capability and write CUDA_ARCH = sm_XXX to CudaArch.mk.
# Used by Makefile when neither CUDA_GENCODE_FLAGS, CUDA_ARCHS nor CUDA_ARCH are
# set externally.
#
# On failure exits 1
set -eu

OUT_FILE="CudaArch.mk"
rm -f "$OUT_FILE"

CAP=""
if command -v nvidia-smi >/dev/null 2>&1; then
    # compute_cap output is e.g. "8.0" / "12.0" — strip the dot to get sm_XXX.
    SMI_CAP=$(nvidia-smi --query-gpu=compute_cap --format=csv,noheader 2>/dev/null \
        | head -n1 \
        | tr -d ' .')
    case "$SMI_CAP" in
        ''|*[!0-9]*) ;; # empty or non-numeric (e.g. driver up but no GPU) — ignore
        *) CAP=$SMI_CAP ;;
    esac
fi
if [ -z "$CAP" ]; then
    echo "[detect_cuda_arch] nvidia-smi probe failed — no GPU arch detected, falling back to major archs." >&2
    exit 1
fi

# Locate nvcc the same way the Makefile/build.rs
NVCC_BIN=""
if command -v nvcc >/dev/null 2>&1; then
    NVCC_BIN=nvcc
elif [ -x /usr/local/cuda/bin/nvcc ]; then
    NVCC_BIN=/usr/local/cuda/bin/nvcc
elif [ -x /opt/cuda/bin/nvcc ]; then
    NVCC_BIN=/opt/cuda/bin/nvcc
fi

# Cap to the highest arch the installed nvcc supports 
SELECTED_CAP=$CAP
if [ -n "$NVCC_BIN" ]; then
    if "$NVCC_BIN" --list-gpu-code >/dev/null 2>&1; then
        NVCC_ARCHS=$("$NVCC_BIN" --list-gpu-code | grep -oE "sm_[0-9]+" | sed 's/sm_//g' | sort -n -u)
    else
        # Fallback to parsing help text
        NVCC_ARCHS=$("$NVCC_BIN" --help | grep -oE "sm_[0-9]+" | sed 's/sm_//g' | sort -n -u)
    fi
    if [ -n "$NVCC_ARCHS" ]; then
        BEST=0
        for arch in $NVCC_ARCHS; do
            if [ "$arch" -le "$CAP" ]; then
                BEST=$arch
            fi
        done
        if [ "$BEST" -eq 0 ]; then
            echo "[detect_cuda_arch] No nvcc-supported CUDA architecture <= capability $CAP!" >&2
            exit 1
        fi
        SELECTED_CAP=$BEST
        if [ "$SELECTED_CAP" -lt "$CAP" ]; then
            echo "[detect_cuda_arch] Warning: capability $CAP detected, capping to highest nvcc-supported sm_$SELECTED_CAP."
        fi
    fi
fi

echo "CUDA_ARCH = sm_$SELECTED_CAP" > "$OUT_FILE"
echo "[detect_cuda_arch] Host GPU compute capability $CAP → CUDA_ARCH = sm_$SELECTED_CAP"
