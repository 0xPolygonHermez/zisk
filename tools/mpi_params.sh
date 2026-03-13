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

# Detect total available threads
TOTAL_THREADS=$(nproc)

# Calculate processes per socket based on GPU grouping strategy
if [ "$NUM_GPUS" -eq 0 ]; then
    # No GPUs: 1 process per socket
    GPUS_PER_SOCKET=0
    PROCS_PER_SOCKET=1
else
    # Warn if GPUs don't divide evenly across sockets
    if [ $((NUM_GPUS % NUM_SOCKETS)) -ne 0 ]; then
        echo "Warning: GPUs ($NUM_GPUS) don't divide evenly across sockets ($NUM_SOCKETS)" >&2
    fi
    
    # Calculate GPUs per socket
    GPUS_PER_SOCKET=$((NUM_GPUS / NUM_SOCKETS))
    
    # Handle edge case: more sockets than GPUs
    if [ "$GPUS_PER_SOCKET" -eq 0 ]; then
        echo "Warning: Fewer GPUs ($NUM_GPUS) than sockets ($NUM_SOCKETS), using 1 process total" >&2
        PROCS_PER_SOCKET=0
    # Determine processes per socket based on GPU grouping strategy:
    # - Prefer groups of 2 GPUs per process
    # - Otherwise groups of 3 GPUs per process
    # - Otherwise 1 process per socket
    elif [ $((GPUS_PER_SOCKET % 2)) -eq 0 ]; then
        PROCS_PER_SOCKET=$((GPUS_PER_SOCKET / 2))
    elif [ $((GPUS_PER_SOCKET % 3)) -eq 0 ]; then
        PROCS_PER_SOCKET=$((GPUS_PER_SOCKET / 3))
    else
        PROCS_PER_SOCKET=1
    fi
fi

# Calculate total processes (X)
if [ "$PROCS_PER_SOCKET" -eq 0 ]; then
    # Edge case: fewer GPUs than sockets, use 1 process total
    NP=1
    PPR=1
else
    NP=$((NUM_SOCKETS * PROCS_PER_SOCKET))
    PPR=$PROCS_PER_SOCKET
fi

# Calculate GPUs per process
if [ "$NUM_GPUS" -gt 0 ] && [ "$NP" -gt 0 ]; then
    GPUS_PER_PROCESS=$((NUM_GPUS / NP))
else
    GPUS_PER_PROCESS=0
fi

# RAYON_NUM_THREADS (Z) - threads per process
RAYON_NUM_THREADS=$((TOTAL_THREADS / NP))

# Ensure at least 1 thread per process
if [ "$RAYON_NUM_THREADS" -lt 1 ]; then
    RAYON_NUM_THREADS=1
fi

# Output results
echo "============================================"
echo "System Configuration:"
echo "============================================"
echo "  Sockets (NUMA nodes): $NUM_SOCKETS"
echo "  GPUs:                 $NUM_GPUS"
echo "  GPUs per socket:      $GPUS_PER_SOCKET"
echo "  GPUs per process:     $GPUS_PER_PROCESS"
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

# Export variables for use in other scripts (requires sourcing: . mpi_params.sh)
export MPI_NP=$NP
export MPI_PPR=$PPR
export MPI_RAYON_NUM_THREADS=$RAYON_NUM_THREADS

# Optionally print as single line for easy copy-paste
echo "Single line: -np $NP -map-by ppr:$PPR:numa --bind-to numa -x RAYON_NUM_THREADS=$RAYON_NUM_THREADS"
