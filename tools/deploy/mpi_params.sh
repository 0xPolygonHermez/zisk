#!/bin/bash

# Script to calculate MPI parameters based on system hardware

set -euo pipefail

SCRIPT_NAME="$(basename "$0")"
source "$(dirname "$0")/deploy_utils.sh"

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
