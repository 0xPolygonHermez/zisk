#!/bin/bash
# End-to-end test of the ziskasm C binding: build a minimal C guest that calls
# ziskos_keccak, run it through ziskemu (which applies the elf2rom symbol
# redirect), and check it emits the correct keccak256 — proving the redirect
# fired and the hand-written .zisk routine ran in the guest's place.
#
# Requires: a RISC-V bare-metal C compiler (riscv64-unknown-elf-gcc or the xpack
# riscv-none-elf-gcc). ziskemu is built here when needed -- see the feature check
# below -- or point ZISKEMU at an existing binary.
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
ZISK="${ZISK_ROOT:-$HERE/../../../..}"
ZISKEMU_DEFAULT="$ZISK/target/release/ziskemu"
ZISKEMU="${ZISKEMU:-$ZISKEMU_DEFAULT}"
CC="${RISCV_CC:-riscv64-unknown-elf-gcc}"
INC="$HERE/../include"                 # zisklib.h
STUBS="$HERE/../src/zisklib_stubs.c"   # ziskos_* stubs (redirected)
LD="$ZISK/ziskbuild/zisk_linker_script.ld"   # the ONE guest linker script
OUT="${OUT:-/tmp/zisk_c_e2e}"; mkdir -p "$OUT"
: > "$OUT/empty.bin"

# The elf2rom symbol redirect lives behind ziskemu's `ziskasm` feature, which is
# OFF by default (emulator/Cargo.toml), so a plain `cargo build --release -p
# ziskemu` yields an emulator that never applies it: the C stub in
# src/zisklib_stubs.c runs, prints "stub reached without redirect", and faults on
# the null guard page, aborting the run before any output is written. That is a
# clear failure, but it blames nothing -- probe for the feature up front instead.
# `-z` is declared unconditionally in the CLI and only its handling is gated, so
# --help cannot tell the two builds apart; the runtime rejection can.
has_ziskasm() {
    [ -x "$1" ] || return 1
    ! "$1" -z /nonexistent.zisk 2>&1 | grep -q "requires building ziskemu with"
}

if ! has_ziskasm "$ZISKEMU"; then
    if [ "$ZISKEMU" != "$ZISKEMU_DEFAULT" ]; then
        echo "ERROR: $ZISKEMU is missing or was built without the 'ziskasm' feature." >&2
        echo "       Rebuild it with:" >&2
        echo "         cargo build --release -p ziskemu --features ziskasm" >&2
        exit 1
    fi
    echo "### building ziskemu with --features ziskasm (the redirect is feature-gated) ..."
    (cd "$ZISK" && cargo build --release -p ziskemu --features ziskasm)
    has_ziskasm "$ZISKEMU" || {
        echo "ERROR: $ZISKEMU still lacks the 'ziskasm' feature after building." >&2
        exit 1
    }
fi

echo "### building minimal C guest (calls ziskos_keccak) ..."
$CC -march=rv64ima -mabi=lp64 -mcmodel=medany -nostdlib -ffreestanding -O2 \
    -I"$HERE" -I"$INC" -T "$LD" \
    -o "$OUT/keccak_e2e.elf" "$HERE/../src/_start.s" "$HERE/main.c" "$STUBS"

echo "### running through ziskemu (elf2rom redirects ziskos_keccak -> zisklib_keccak) ..."
# Keep ziskemu's output. If the redirect does not fire, the stub prints a diagnostic
# naming the unresolved symbol and then faults on the null guard page, so ziskemu
# aborts without writing any output. Discarding that (and letting `set -e` end the
# script) would hide the one line that says what actually went wrong.
if ! "$ZISKEMU" -e "$OUT/keccak_e2e.elf" -i "$OUT/empty.bin" -o "$OUT/out.bin" \
        >"$OUT/emu.log" 2>&1; then
    echo "FAIL - ziskemu aborted. Its output:" >&2
    sed 's/^/    /' "$OUT/emu.log" >&2
    echo "A 'stub reached without redirect' line above means elf2rom did not resolve" >&2
    echo "the symbol: the guest ELF was stripped, or ziskemu lacks 'ziskasm'." >&2
    exit 1
fi

GOT=$(xxd -p -c32 "$OUT/out.bin" | head -1)
EXP="c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
echo "  expected keccak256(\"\") = $EXP"
echo "  guest emitted           = $GOT"
if [ "$GOT" = "$EXP" ]; then
    echo "PASS — redirect fired and the .zisk keccak produced the correct hash."
else
    echo "FAIL — got $GOT"
    echo "  The run completed, so the redirect DID fire (an unredirected stub aborts"
    echo "  ziskemu before this point). A wrong hash here means the .zisk keccak"
    echo "  routine itself is producing an incorrect result."
    exit 1
fi
