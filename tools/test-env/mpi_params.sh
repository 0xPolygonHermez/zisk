#!/bin/bash

# Script to calculate MPI parameters based on system hardware

set -euo pipefail

SCRIPT_NAME="$(basename "$0")"
source "$(dirname "$0")/deploy_utils.sh"

# =============================================================================
# mpi_params [SUMMARY]
# Computes optimal MPI parameters for the current machine and exports:
#   MPI_NP                — total number of processes (-np)
#   MPI_PPR               — processes per NUMA node (-map-by ppr:MPI_PPR:numa)
#   MPI_RAYON_NUM_THREADS — threads per process (-x RAYON_NUM_THREADS=…)
#
# SUMMARY: optional boolean (true/false). When true, prints the System
#          Configuration and MPI Parameters tables to stdout. Defaults to false.
#
# GPU grouping strategy (when GPUs are present):
#   - Prefer groups of 2 GPUs per process, else groups of 3, else 1 process per socket.
#   - If GPUs < sockets, fall back to 1 process total.
# =============================================================================
mpi_params() {
  local summary="${1:-false}"
  # Detect number of sockets (NUMA nodes)
  local num_sockets
  num_sockets=$(lscpu 2>/dev/null | grep "^Socket(s):" | awk '{print $2}')
  if [[ -z "$num_sockets" || "$num_sockets" -eq 0 ]]; then
    num_sockets=$(numactl --hardware 2>/dev/null | grep "available:" | awk '{print $2}')
  fi
  if [[ -z "$num_sockets" || "$num_sockets" -eq 0 ]]; then
    err "mpi_params: could not detect number of sockets/NUMA nodes." true
    exit 1
  fi

  # Detect number of GPUs
  local num_gpus=0
  if command -v nvidia-smi &>/dev/null; then
    num_gpus=$(nvidia-smi -L 2>/dev/null | wc -l)
  fi

  # Detect total available threads
  local total_threads
  total_threads=$(nproc)

  # Calculate processes per socket
  local gpus_per_socket=0 procs_per_socket=1
  if [[ "$num_gpus" -gt 0 ]]; then
    if [[ $((num_gpus % num_sockets)) -ne 0 ]]; then
      warn "GPUs (${num_gpus}) don't divide evenly across sockets (${num_sockets})."
    fi
    gpus_per_socket=$((num_gpus / num_sockets))
    if [[ "$gpus_per_socket" -eq 0 ]]; then
      warn "Fewer GPUs (${num_gpus}) than sockets (${num_sockets}), using 1 process total."
      procs_per_socket=0
    elif [[ $((gpus_per_socket % 2)) -eq 0 ]]; then
      procs_per_socket=$((gpus_per_socket / 2))
    elif [[ $((gpus_per_socket % 3)) -eq 0 ]]; then
      procs_per_socket=$((gpus_per_socket / 3))
    else
      procs_per_socket=1
    fi
  fi

  # Calculate NP and PPR
  local np ppr
  if [[ "$procs_per_socket" -eq 0 ]]; then
    np=1
    ppr=1
  else
    np=$((num_sockets * procs_per_socket))
    ppr=$procs_per_socket
  fi

  # Calculate GPUs per process (informational only)
  local gpus_per_process=0
  if [[ "$num_gpus" -gt 0 && "$np" -gt 0 ]]; then
    gpus_per_process=$(( num_gpus / np ))
  fi

  # Calculate RAYON_NUM_THREADS (at least 1)
  local rayon_num_threads=$(( total_threads / np ))
  [[ "$rayon_num_threads" -lt 1 ]] && rayon_num_threads=1

  if [[ "$summary" == "true" ]]; then
    echo "============================================"
    echo "System Configuration:"
    echo "============================================"
    echo "  Sockets (NUMA nodes): $num_sockets"
    echo "  GPUs:                 $num_gpus"
    echo "  GPUs per socket:      $gpus_per_socket"
    echo "  GPUs per process:     $gpus_per_process"
    echo "  Total threads:        $total_threads"
    echo ""
    echo "============================================"
    echo "MPI Parameters:"
    echo "============================================"
    echo "  Total processes (-np):           $np"
    echo "  Processes per NUMA (ppr):        $ppr"
    echo "  Threads per process (RAYON):     $rayon_num_threads"
    echo ""
  fi

  export MPI_NP=$np
  export MPI_PPR=$ppr
  export MPI_RAYON_NUM_THREADS=$rayon_num_threads
}

mpi_params true

echo "============================================"
echo "Generated mpirun flags:"
echo "============================================"
echo "  -np $MPI_NP \\"
echo "  -map-by ppr:$MPI_PPR:numa \\"
echo "  --bind-to numa \\"
echo "  -x RAYON_NUM_THREADS=$MPI_RAYON_NUM_THREADS"
echo ""
echo "Single line: -np $MPI_NP -map-by ppr:$MPI_PPR:numa --bind-to numa -x RAYON_NUM_THREADS=$MPI_RAYON_NUM_THREADS"