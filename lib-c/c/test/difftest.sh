#!/bin/bash
# Checks that the lib-c precompile entry points of the working tree give byte-identical results
# to a reference commit (default: HEAD), with test/difftest.cpp.
#
# Usage: lib-c/c/test/difftest.sh [base_commit] [filter]
#
# The reference library is built from base_commit in a temporary git worktree, and both
# libraries are built from clean, since the lib-c Makefile does not track every header
# dependency. The test program is always the working tree one, so it can be run against older
# commits as long as they export the tested entry points. Set KEEP=1 to keep the temporary
# directory (reference records, both libraries and both test binaries).
set -e

BASE=${1:-HEAD}
FILTER=${2:-}
C=$(cd "$(dirname "$0")/.." && pwd)
ROOT=$(git -C "$C" rev-parse --show-toplevel)
WORK=$(mktemp -d)
JOBS=$(nproc)

cleanup() {
    git -C "$ROOT" worktree remove --force "$WORK/base" > /dev/null 2>&1 || true
    if [ -z "$KEEP" ]; then rm -rf "$WORK"; else echo "kept $WORK"; fi
}
trap cleanup EXIT

# Builds the lib-c of source dir $1 into $WORK/$2, and the test program linked with it
build() {
    local src=$1 name=$2
    make -C "$src" -j"$JOBS" BUILD_DIR="$WORK/$name/build" LIB_DIR="$WORK/$name/lib" \
        "$WORK/$name/lib/libziskc.a" > "$WORK/$name.build.log" 2>&1 \
        || { tail -30 "$WORK/$name.build.log"; exit 1; }
    g++ -O2 -std=c++17 -I"$C/src" "$C/test/difftest.cpp" "$WORK/$name/lib/libziskc.a" \
        -lgmpxx -lgmp -lpthread -o "$WORK/$name/difftest"
}

echo "Building reference lib-c from $BASE ($(git -C "$ROOT" rev-parse --short "$BASE"))"
git -C "$ROOT" worktree add --detach "$WORK/base" "$BASE" > /dev/null 2>&1
build "$WORK/base/lib-c/c" ref

echo "Building working tree lib-c"
build "$C" new

echo "Generating reference records"
mkdir -p "$WORK/records"
"$WORK/ref/difftest" gen "$WORK/records" $FILTER > /dev/null

echo "Checking"
# The library prints some expected errors (e.g. division by zero cases) to stdout
"$WORK/new/difftest" check "$WORK/records" $FILTER > /dev/null
