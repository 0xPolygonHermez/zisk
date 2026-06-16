#!/usr/bin/env bash
#
# DEPRECATED: superseded by build-libziskos-isolated.sh, which builds
# libziskos_staticlib.a, finalizes the fat-LTO bitcode into a native object,
# isolates the allocator from the host application, and runs the same
# "no std bundled" + required-symbol checks this script used to do.
#
# Kept as a thin wrapper so existing callers keep working.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${SCRIPT_DIR}/build-libziskos-isolated.sh" "$@"
