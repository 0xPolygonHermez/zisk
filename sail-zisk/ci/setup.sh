#!/usr/bin/env bash
# The tools sail-zisk's checks need, beyond what tools/test-env/install_deps.sh
# installs (Rust, a C compiler, CMake): Sail (through opam), z3, elan, the
# xPack RISC-V GCC and riscv-arch-test, at the versions the README lists.
# Each step is skipped when its result is already there, so a CI cache of
# ~/.opam, ~/.elan and $SAIL_ZISK_TOOLS makes a rerun quick.
#
# Writes to $GITHUB_ENV, when set, RISCV_GCC and ARCH_TEST, and to
# $GITHUB_PATH the opam and elan bin directories.
set -euo pipefail

SAIL_VERSION=0.20.3
XPACK_GCC_VERSION=14.3.0-1
ARCH_TEST_TAG=3.10.0
TOOLS=${SAIL_ZISK_TOOLS:-$HOME/sail-zisk-tools}
mkdir -p "$TOOLS"

SUDO=""
if [ "$(id -u)" != 0 ]; then SUDO=sudo; fi

echo "::group::apt packages"
$SUDO apt-get update -q
$SUDO apt-get install -y -q --no-install-recommends opam z3 libgmp-dev zlib1g-dev pkg-config m4 \
  python3 git curl xz-utils ca-certificates build-essential cmake unzip bubblewrap
echo "::endgroup::"

echo "::group::Sail $SAIL_VERSION"
if ! [ -x "$HOME/.opam/default/bin/sail" ] || \
   ! "$HOME/.opam/default/bin/sail" --version | grep -q "Sail $SAIL_VERSION"; then
  [ -d "$HOME/.opam" ] || opam init -y --bare --disable-sandboxing
  opam switch list | grep -q default || opam switch create -y default ocaml-base-compiler.5.2.1
  opam install -y --switch default "sail.$SAIL_VERSION"
fi
"$HOME/.opam/default/bin/sail" --version
echo "::endgroup::"

echo "::group::elan"
if ! [ -x "$HOME/.elan/bin/lake" ]; then
  curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh | sh -s -- -y --default-toolchain none
fi
"$HOME/.elan/bin/elan" --version
echo "::endgroup::"

echo "::group::xPack RISC-V GCC $XPACK_GCC_VERSION"
GCC_DIR="$TOOLS/xpack-riscv-none-elf-gcc-$XPACK_GCC_VERSION"
if ! [ -x "$GCC_DIR/bin/riscv-none-elf-gcc" ]; then
  curl -sSfL "https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack/releases/download/v$XPACK_GCC_VERSION/xpack-riscv-none-elf-gcc-$XPACK_GCC_VERSION-linux-x64.tar.gz" \
    | tar -xz -C "$TOOLS"
fi
"$GCC_DIR/bin/riscv-none-elf-gcc" --version | head -1
echo "::endgroup::"

echo "::group::riscv-arch-test $ARCH_TEST_TAG"
ARCH_DIR="$TOOLS/riscv-arch-test"
if ! [ -d "$ARCH_DIR/.git" ]; then
  git clone -q --depth 1 --branch "$ARCH_TEST_TAG" https://github.com/riscv-non-isa/riscv-arch-test "$ARCH_DIR"
fi
git -C "$ARCH_DIR" describe --tags
echo "::endgroup::"

if [ -n "${GITHUB_PATH:-}" ]; then
  echo "$HOME/.opam/default/bin" >> "$GITHUB_PATH"
  echo "$HOME/.elan/bin" >> "$GITHUB_PATH"
fi
if [ -n "${GITHUB_ENV:-}" ]; then
  echo "RISCV_GCC=$GCC_DIR/bin/riscv-none-elf-gcc" >> "$GITHUB_ENV"
  echo "ARCH_TEST=$ARCH_DIR" >> "$GITHUB_ENV"
  # Sail finds its library through opam's switch
  echo "OPAM_SWITCH_PREFIX=$HOME/.opam/default" >> "$GITHUB_ENV"
fi
