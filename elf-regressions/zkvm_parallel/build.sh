#!/bin/bash
# Regenerate the committed ELFs of this directory, fork-join guests of the ziskasm C binding:
# zkvm_parallel.elf (ziskasm/lang/c/example/zkvm_parallel_guest.c, SAXPY over four tasks) and
# zkvm_parallel_fcsr.elf (zkvm_parallel_fcsr_guest.c, tasks that change the FCSR). Checked by
# emulator/tests/zkvm_parallel.rs and, on the assembly emulator, by
# integration-tests/tests/asm_fork_join.rs. Not built by ../scripts/build.sh, which builds
# assembly sources.
set -euo pipefail
readonly DIR="$(cd "$(dirname "$0")" && pwd)"
readonly C="$DIR/../../ziskasm/lang/c"
readonly CC="${RISCV_CC:-riscv64-unknown-elf-gcc}"
# Older toolchains have no separate zicsr extension
MARCH=rv64ima_zicsr
"$CC" -march=$MARCH -mabi=lp64 -E -x c /dev/null >/dev/null 2>&1 || MARCH=rv64ima
"$CC" -march=$MARCH -mabi=lp64 -mcmodel=medany -nostdlib -ffreestanding -O2 \
    -I"$C/include" -T "$DIR/../../ziskbuild/zisk_linker_script.ld" -Wl,--gc-sections \
    -o "$DIR/zkvm_parallel.elf" "$C/src/_start.s" "$C/example/zkvm_parallel_guest.c" "$C/src/zkvm_calls.s"
"$CC" -march=$MARCH -mabi=lp64 -mcmodel=medany -nostdlib -ffreestanding -O2 \
    -I"$C/include" -T "$DIR/../../ziskbuild/zisk_linker_script.ld" -Wl,--gc-sections \
    -o "$DIR/zkvm_parallel_fcsr.elf" "$C/src/_start.s" "$C/example/zkvm_parallel_fcsr_guest.c" "$C/src/zkvm_calls.s"
