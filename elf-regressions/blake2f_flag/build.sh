#!/bin/bash
# Regenerate the committed ELF of this directory: the zkvm_blake2f final-flag guest of the
# ziskasm C binding (ziskasm/lang/c/example/blake2f_flag_guest.c), checked by
# emulator/tests/blake2f_flag.rs. Not built by ../scripts/build.sh, which builds
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
    -o "$DIR/blake2f_flag.elf" "$C/src/_start.s" "$C/example/blake2f_flag_guest.c" "$C/src/zkvm_calls.s"
