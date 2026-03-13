#!/bin/bash

# Script to calculate MPI parameters based on system hardware
# X: total processes (-np)
# Y: processes per NUMA (-map-by ppr:Y:numa)
# Z: threads per process (-x RAYON_NUM_THREADS=Z)

set -e

# Detect number of sockets (NUMA nodes)
NUM_SOCKETS=$(lscpu | grep "^Socket(s):" | awk '{print $2}')
if [ -z "$NUM_SOCKETS" ] || [ "$NUM_SOCKETS" -eq 0 ]; then
    # Fallback to numactl
    NUM_SOCKETS=$(numactl --hardware 2>/dev/null | grep "available:" | awk '{print $2}')
fi

if [ -z "$NUM_SOCKETS" ] || [ "$NUM_SOCKETS" -eq 0 ]; then
    echo "Error: Could not detect number of sockets" >&2
    exit 1
fi

# Detect number of GPUs
if command -v nvidia-smi &> /dev/null; then
    NUM_GPUS=$(nvidia-smi -L 2>/dev/null | wc -l)
else
    NUM_GPUS=0
fi

if [ "$NUM_GPUS" -eq 0 ]; then
    echo "Error: No GPUs detected" >&2
    exit 1
fi

# Detect total available threads
TOTAL_THREADS=$(nproc)

# Calculate GPUs per socket
GPUS_PER_SOCKET=$((NUM_GPUS / NUM_SOCKETS))

# Determine processes per socket based on GPU grouping strategy:
# - Prefer groups of 2 GPUs per process
# - Otherwise groups of 3 GPUs per process
# - Otherwise 1 process per socket
if [ "$GPUS_PER_SOCKET" -gt 0 ] && [ $((GPUS_PER_SOCKET % 2)) -eq 0 ]; then
    PROCS_PER_SOCKET=$((GPUS_PER_SOCKET / 2))
elif [ "$GPUS_PER_SOCKET" -gt 0 ] && [ $((GPUS_PER_SOCKET % 3)) -eq 0 ]; then
    PROCS_PER_SOCKET=$((GPUS_PER_SOCKET / 3))
else
    PROCS_PER_SOCKET=1
fi

# Calculate total processes (X)
NP=$((NUM_SOCKETS * PROCS_PER_SOCKET))

# PPR value (Y) - processes per NUMA
PPR=$PROCS_PER_SOCKET

# RAYON_NUM_THREADS (Z) - threads per process
RAYON_NUM_THREADS=$((TOTAL_THREADS / NP))

# Output results
echo "============================================"
echo "System Configuration:"
echo "============================================"
echo "  Sockets (NUMA nodes): $NUM_SOCKETS"
echo "  GPUs:                 $NUM_GPUS"
echo "  GPUs per socket:      $GPUS_PER_SOCKET"
echo "  Total threads:        $TOTAL_THREADS"
echo ""
echo "============================================"
echo "MPI Parameters:"
echo "============================================"
echo "  Total processes (-np):           $NP"
echo "  Processes per NUMA (ppr):        $PPR"
echo "  Threads per process (RAYON):     $RAYON_NUM_THREADS"
echo ""
echo "============================================"
echo "Generated mpirun flags:"
echo "============================================"
echo "  -np $NP \\"
echo "  -map-by ppr:$PPR:numa \\"
echo "  --bind-to numa \\"
echo "  -x RAYON_NUM_THREADS=$RAYON_NUM_THREADS"
echo ""

# Export variables for use in other scripts
export MPI_NP=$NP
export MPI_PPR=$PPR
export MPI_RAYON_NUM_THREADS=$RAYON_NUM_THREADS

# Optionally print as single line for easy copy-paste
echo "Single line: -np $NP -map-by ppr:$PPR:numa --bind-to numa -x RAYON_NUM_THREADS=$RAYON_NUM_THREADS"
