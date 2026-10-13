#!/bin/bash
# Regenerate the committed ELFs of this directory from their sources. They are not
# built by ../scripts/build.sh, whose harness expects every ELF to run successfully;
# emulator/tests/failed_execution.rs checks them instead.
set -euo pipefail
readonly DIR="$(cd "$(dirname "$0")" && pwd)"
readonly PREFIX="${RISCV_PREFIX:-riscv64-unknown-elf-}"
for s in "$DIR"/*.s; do
    n="$(basename "$s" .s)"
    "${PREFIX}as" -march=rv64ima_zicsr -mabi=lp64 -o "$DIR/$n.o" "$s"
    "${PREFIX}ld" -m elf64lriscv -T "$DIR/failed_execution.ld" -o "$DIR/$n.elf" "$DIR/$n.o"
    rm -f "$DIR/$n.o"
done
