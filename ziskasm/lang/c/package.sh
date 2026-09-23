#!/bin/bash
# Build the distributable ZisK guest archive (EF zkVM standard §9): a static
# library providing _start, the I/O functions and every accelerator stub, plus
# the public headers and the guest linker script.
#
#   ./package.sh                       # -> dist/
#   PREFIX=/tmp/out ./package.sh       # install elsewhere
#   TARBALL=1 ./package.sh             # also produce dist.tar.gz
#   ZISK_TOOLCHAIN_PREFIX=riscv-none-elf- ./package.sh   # xPack toolchain
#
# NOTE: the archive is NOT standalone-functional. Every accelerator/I-O symbol in
# it is a stub whose entry `elf2rom` rewrites to a hand-written .zisk routine at
# transpile time (_start and the DMA-backed mem* routines are real code). A guest that links it and is then run through a ziskemu/cargo-zisk built
# WITHOUT the `ziskasm` feature reaches the stub bodies, which deliberately fail
# hard (diagnostic + fault) rather than returning wrong answers.
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HERE/dist}"
BUILD="${BUILD:-$HERE/build-pkg}"
CC_PREFIX="${ZISK_TOOLCHAIN_PREFIX:-riscv64-unknown-elf-}"

if ! command -v "${CC_PREFIX}gcc" >/dev/null 2>&1; then
    echo "ERROR: ${CC_PREFIX}gcc not found on PATH." >&2
    echo "       Install a RISC-V bare-metal toolchain, or set" >&2
    echo "       ZISK_TOOLCHAIN_PREFIX (e.g. riscv-none-elf- for xPack)." >&2
    exit 1
fi

echo "### configuring (${CC_PREFIX}gcc -> $PREFIX) ..."
cmake -S "$HERE" -B "$BUILD" \
    -DCMAKE_TOOLCHAIN_FILE="$HERE/zisk-guest-toolchain.cmake" \
    -DZISK_TOOLCHAIN_PREFIX="$CC_PREFIX" \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DCMAKE_BUILD_TYPE=Release >/dev/null

echo "### building ..."
cmake --build "$BUILD" --parallel >/dev/null

echo "### installing ..."
rm -rf "$PREFIX"
cmake --install "$BUILD" >/dev/null

echo
echo "Staged in $PREFIX:"
(cd "$PREFIX" && find . -type f | sort | sed 's/^\./  /')

# Sanity-check the archive really carries the whole §9 surface.
echo
echo "### checking the archive exports the required symbols ..."
AR_FILE="$(find "$PREFIX" -name 'libzisklib_c.a' | head -1)"
MISSING=0
for sym in _start read_input write_output zkvm_keccak256 zkvm_u256_add \
           memcpy memmove memcmp memset; do
    if "${CC_PREFIX}nm" "$AR_FILE" 2>/dev/null | grep -qE "^[0-9a-f]* T $sym$"; then
        echo "  OK   $sym"
    else
        echo "  MISSING $sym" >&2; MISSING=1
    fi
done
[ "$MISSING" -eq 0 ] || { echo "ERROR: archive is incomplete." >&2; exit 1; }

if [ -n "$TARBALL" ]; then
    tar -czf "$HERE/dist.tar.gz" -C "$(dirname "$PREFIX")" "$(basename "$PREFIX")"
    echo
    echo "Tarball: $HERE/dist.tar.gz"
fi

echo
echo "Link a guest with:"
echo "  ${CC_PREFIX}gcc -march=rv64ima -mabi=lp64 -mcmodel=medany -nostdlib -ffreestanding \\"
echo "      -I$PREFIX/include -T $PREFIX/share/zisk/zisk_linker_script.ld \\"
echo "      -o guest.elf guest.c $AR_FILE"
echo "Do NOT strip the result: elf2rom resolves the stubs by symbol name."
