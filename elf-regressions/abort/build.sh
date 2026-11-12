#!/bin/bash
# Regenerate the committed ELFs of this directory: the abort guest of the ziskasm C
# binding (ziskasm/lang/c/example/abort_guest.c), and abort_control.elf, the same
# guest built with RETURN_NORMALLY. Checked by emulator/tests/failed_execution.rs.
# Not built by ../scripts/build.sh, whose harness expects every ELF to run
# successfully.
set -euo pipefail
readonly DIR="$(cd "$(dirname "$0")" && pwd)"
readonly C="$DIR/../../ziskasm/lang/c"
readonly CC="${RISCV_CC:-riscv64-unknown-elf-gcc}"
# Older toolchains have no separate zicsr extension
MARCH=rv64ima_zicsr
"$CC" -march=$MARCH -mabi=lp64 -E -x c /dev/null >/dev/null 2>&1 || MARCH=rv64ima
build() {
    "$CC" -march=$MARCH -mabi=lp64 -mcmodel=medany -nostdlib -ffreestanding -O2 "$@" \
        -I"$C/include" -T "$DIR/../../ziskbuild/zisk_linker_script.ld" -Wl,--gc-sections \
        "$C/src/_start.s" "$C/example/abort_guest.c" "$C/src/zkvm_calls.s"
}
build -o "$DIR/abort.elf"
build -DRETURN_NORMALLY -o "$DIR/abort_control.elf"
