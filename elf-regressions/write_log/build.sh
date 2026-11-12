#!/bin/bash
# Regenerate the committed ELF of this directory: the write_log guest of the
# ziskasm C binding (ziskasm/lang/c/example/write_log_guest.c and
# write_log_thunk.c), checked by emulator/tests/write_log.rs. Not built by
# ../scripts/build.sh, which builds assembly sources.
set -euo pipefail
readonly DIR="$(cd "$(dirname "$0")" && pwd)"
readonly C="$DIR/../../ziskasm/lang/c"
readonly CC="${RISCV_CC:-riscv64-unknown-elf-gcc}"
# Older toolchains have no separate zicsr extension
MARCH=rv64ima_zicsr
"$CC" -march=$MARCH -mabi=lp64 -E -x c /dev/null >/dev/null 2>&1 || MARCH=rv64ima
"$CC" -march=$MARCH -mabi=lp64 -mcmodel=medany -nostdlib -ffreestanding -O2 \
    -I"$C/include" -T "$DIR/../../ziskbuild/zisk_linker_script.ld" -Wl,--gc-sections \
    -o "$DIR/write_log.elf" "$C/src/_start.s" "$C/example/write_log_guest.c" \
    "$C/example/write_log_thunk.c" "$C/src/zkvm_calls.s"
