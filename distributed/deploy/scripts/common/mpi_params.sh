#!/usr/bin/env bash
# mpi_params.sh — auto-detect optimal MPI parameters for the current host.
#
# Usage:
#   ./mpi_params.sh [--quiet]
#
# Pure script: no side effects, no file writes. Prints KEY=VAL lines to stdout.
# By default also prints a human-readable system + MPI table to stderr; pass
# --quiet to suppress that for automated callers. Both bash install scripts
# (via `eval "$(./mpi_params.sh)"`) and Ansible (via `ansible.builtin.script`
# + `set_fact`) consume the stdout output unchanged.
#
# Output keys (stdout):
#   MPI_NP                 — total MPI processes (-np)
#   MPI_PPR                — processes per NUMA node (-map-by ppr:N:numa)
#   MPI_RAYON_NUM_THREADS  — threads per process (-x RAYON_NUM_THREADS=N)
#   MPI_NUM_SOCKETS        — informational
#   MPI_NUM_GPUS           — informational
#   MPI_TOTAL_THREADS      — informational
#
# Strategy when GPUs are present:
#   Prefer 2 GPUs/process, else 3, else 1 process per socket.
#   GPUs < sockets → fall back to 1 process total.
# Without GPUs:
#   1 process per socket.

set -euo pipefail

SUMMARY=true
while [[ $# -gt 0 ]]; do
    case "$1" in
        --quiet) SUMMARY=false; shift ;;
        *) echo "mpi_params: unknown flag: $1" >&2; exit 1 ;;
    esac
done

_detect_linux() {
    local sockets gpus threads
    sockets=$(lscpu 2>/dev/null | awk '/^Socket\(s\):/ {print $2}')
    if [[ -z "${sockets:-}" || "$sockets" -eq 0 ]]; then
        sockets=$(numactl --hardware 2>/dev/null | awk '/^available:/ {print $2}')
    fi
    [[ -z "${sockets:-}" || "$sockets" -eq 0 ]] && {
        echo "mpi_params: cannot detect socket count on Linux" >&2
        exit 1
    }

    gpus=0
    if command -v nvidia-smi &>/dev/null; then
        gpus=$(nvidia-smi -L 2>/dev/null | wc -l)
    fi

    threads=$(nproc)

    NUM_SOCKETS=$sockets
    NUM_GPUS=$gpus
    TOTAL_THREADS=$threads
}

_detect_macos() {
    # macOS has no NUMA. Treat each physical CPU package as a "socket".
    local sockets threads
    sockets=$(sysctl -n hw.packages 2>/dev/null || echo 1)
    [[ -z "${sockets:-}" || "$sockets" -eq 0 ]] && sockets=1

    # No CUDA on Apple Silicon, irrelevant for prove workloads on Intel macOS.
    # Always 0 — auto-detect will fall back to 1 process per socket.
    threads=$(sysctl -n hw.logicalcpu 2>/dev/null || sysctl -n hw.ncpu)

    NUM_SOCKETS=$sockets
    NUM_GPUS=0
    TOTAL_THREADS=$threads
}

case "$(uname -s)" in
    Linux)  _detect_linux ;;
    Darwin) _detect_macos ;;
    *) echo "mpi_params: unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac

# Compute processes per socket from GPU layout.
gpus_per_socket=0
procs_per_socket=1
if [[ "$NUM_GPUS" -gt 0 ]]; then
    if [[ $((NUM_GPUS % NUM_SOCKETS)) -ne 0 ]]; then
        echo "mpi_params: warning: GPUs (${NUM_GPUS}) don't divide evenly across sockets (${NUM_SOCKETS})." >&2
    fi
    gpus_per_socket=$((NUM_GPUS / NUM_SOCKETS))
    if [[ "$gpus_per_socket" -eq 0 ]]; then
        echo "mpi_params: warning: fewer GPUs (${NUM_GPUS}) than sockets (${NUM_SOCKETS}); using 1 process total." >&2
        procs_per_socket=0
    elif [[ $((gpus_per_socket % 2)) -eq 0 ]]; then
        procs_per_socket=$((gpus_per_socket / 2))
    elif [[ $((gpus_per_socket % 3)) -eq 0 ]]; then
        procs_per_socket=$((gpus_per_socket / 3))
    else
        procs_per_socket=1
    fi
fi

if [[ "$procs_per_socket" -eq 0 ]]; then
    np=1
    ppr=1
else
    np=$((NUM_SOCKETS * procs_per_socket))
    ppr=$procs_per_socket
fi

rayon=$((TOTAL_THREADS / np))
[[ "$rayon" -lt 1 ]] && rayon=1

# Informational only — printed inside the summary table.
gpus_per_process=0
if [[ "$NUM_GPUS" -gt 0 && "$np" -gt 0 ]]; then
    gpus_per_process=$((NUM_GPUS / np))
fi

if $SUMMARY; then
    {
        echo "============================================"
        echo "System Configuration:"
        echo "============================================"
        echo "  Sockets (NUMA nodes): ${NUM_SOCKETS}"
        echo "  GPUs:                 ${NUM_GPUS}"
        echo "  GPUs per socket:      ${gpus_per_socket}"
        echo "  GPUs per process:     ${gpus_per_process}"
        echo "  Total threads:        ${TOTAL_THREADS}"
        echo ""
        echo "============================================"
        echo "MPI Parameters:"
        echo "============================================"
        echo "  Total processes (-np):           ${np}"
        echo "  Processes per NUMA (ppr):        ${ppr}"
        echo "  Threads per process (RAYON):     ${rayon}"
        echo ""
    } >&2
fi

cat <<EOF
MPI_NP=${np}
MPI_PPR=${ppr}
MPI_RAYON_NUM_THREADS=${rayon}
MPI_NUM_SOCKETS=${NUM_SOCKETS}
MPI_NUM_GPUS=${NUM_GPUS}
MPI_TOTAL_THREADS=${TOTAL_THREADS}
EOF
