#!/bin/bash
# Regenerate ../prebuilt-elfs/cpp_static_init.elf from main.cpp + start.S.
#
# Not part of ./scripts/build.sh (which only assembles .s files inside Docker):
# this needs a C++ cross-compiler and reaches the real ZisK linker script at
# ../../ziskbuild/zisk_linker_script.ld, so it runs on the host and is invoked
# by hand whenever the sources change. The resulting ELF is committed so
# `cargo test -p ziskemu --test cpp_static_init` needs no cross toolchain.
#
# Requirements:
#   * riscv64-unknown-elf-g++  (Debian/Ubuntu: gcc-riscv64-unknown-elf)
#   * the `zisk` Rust toolchain, for rust-lld (the production guest linker)
#
# Override the linker with LD=... to cross-check another one, e.g.
#   LD=riscv64-unknown-elf-ld ./build.sh

set -euo pipefail

readonly DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly LINKER_SCRIPT="${DIR}/../../ziskbuild/zisk_linker_script.ld"
readonly OUT="${DIR}/../prebuilt-elfs/cpp_static_init.elf"

readonly CXX="${CXX:-riscv64-unknown-elf-g++}"
readonly LD="${LD:-$HOME/.rustup/toolchains/zisk/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld}"

# Match the ISA/ABI and code model documented for a C++ ZisK host
# (ziskos-staticlib/README.md): RAM lives at high addresses, so medany.
readonly CXXFLAGS=(
    -march=rv64ima -mabi=lp64 -mcmodel=medany
    -ffreestanding -fno-exceptions -fno-rtti -O1
)

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CXX" "${CXXFLAGS[@]}" -c "${DIR}/main.cpp" -o "${TMP}/main.o"
"$CXX" -march=rv64ima -mabi=lp64 -c "${DIR}/start.S" -o "${TMP}/start.o"

# rust-lld needs the `-flavor gnu` prefix; a plain GNU ld does not accept it.
LD_ARGS=()
if [[ "$(basename "$LD")" == "rust-lld" ]]; then
    LD_ARGS+=(-flavor gnu)
fi
"$LD" "${LD_ARGS[@]}" -T "$LINKER_SCRIPT" "${TMP}/start.o" "${TMP}/main.o" -o "$OUT"

echo "Wrote ${OUT}"
riscv64-unknown-elf-readelf -lW "$OUT" | sed -n '/Program Headers/,$p'
