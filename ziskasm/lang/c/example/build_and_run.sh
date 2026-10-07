#!/bin/bash
# End-to-end test of the ziskasm C binding: build a minimal C guest that calls
# zkvm_keccak256, run it through ziskemu (whose transpiler turns the zkvmcall into
# a jump to the .zisk routine), and check it emits the correct keccak256 — proving
# the hand-written .zisk routine ran in the guest's place.
#
# Requires: a RISC-V bare-metal C compiler (riscv64-unknown-elf-gcc or the xpack
# riscv-none-elf-gcc). ziskemu is built here when it is missing, or point ZISKEMU at
# an existing binary.
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
ZISK="${ZISK_ROOT:-$HERE/../../../..}"
ZISKEMU_DEFAULT="$ZISK/target/release/ziskemu"
ZISKEMU="${ZISKEMU:-$ZISKEMU_DEFAULT}"
CC="${RISCV_CC:-riscv64-unknown-elf-gcc}"
INC="$HERE/../include"                 # zkvm_accelerators.h
CALLS="$HERE/../src/zkvm_calls.s"      # zkvmcall thunks
LD="$ZISK/ziskbuild/zisk_linker_script.ld"   # the ONE guest linker script
OUT="${OUT:-/tmp/zisk_c_e2e}"; mkdir -p "$OUT"
: > "$OUT/empty.bin"

if [ ! -x "$ZISKEMU" ]; then
    if [ "$ZISKEMU" != "$ZISKEMU_DEFAULT" ]; then
        echo "ERROR: $ZISKEMU does not exist." >&2
        exit 1
    fi
    echo "### building ziskemu ..."
    (cd "$ZISK" && cargo build --release -p ziskemu)
fi

echo "### building minimal C guest (calls zkvm_keccak256) ..."
$CC -march=rv64ima_zicsr -mabi=lp64 -mcmodel=medany -nostdlib -ffreestanding -O2 \
    -I"$HERE" -I"$INC" -T "$LD" \
    -o "$OUT/keccak_e2e.elf" "$HERE/../src/_start.s" "$HERE/ef_keccak_guest.c" "$CALLS"

echo "### running through ziskemu (zkvm_keccak256 -> ziskasm_zkvm_keccak256) ..."
if ! "$ZISKEMU" -e "$OUT/keccak_e2e.elf" -i "$OUT/empty.bin" -o "$OUT/out.bin" \
        >"$OUT/emu.log" 2>&1; then
    echo "FAIL - ziskemu aborted. Its output:" >&2
    sed 's/^/    /' "$OUT/emu.log" >&2
    exit 1
fi

GOT=$(xxd -p -c32 "$OUT/out.bin" | head -1)
EXP="c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
echo "  expected keccak256(\"\") = $EXP"
echo "  guest emitted           = $GOT"
if [ "$GOT" = "$EXP" ]; then
    echo "PASS — the zkvmcall reached the .zisk keccak, which produced the correct hash."
else
    echo "FAIL — got $GOT: the .zisk keccak routine produced an incorrect result."
    exit 1
fi
