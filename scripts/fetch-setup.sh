#!/usr/bin/env bash
#
# Fetch the cached proving key from https://storage.googleapis.com/zisk-setup/
# if the cloud sidecar matches the local input hash. No build is attempted on
# miss — this script only downloads. To build locally, use build-setup.sh.
#
# Cache lookup mirrors build-setup.sh (same input hash, same bucket layout).
#
# Modes
# -----
#   (default)      compute input hash and download on match
#   --force        skip the hash compare; download whatever sidecar+tarball
#                  the bucket has for the current VERSION (handy when frops
#                  bins aren't built yet and you trust the latest upload)
#
# Usage:
#   fetch-setup.sh [--build-dir DIR] [--skip-compile-pil] [--force] [--release]
#
#   --build-dir DIR      Where to extract provingKey/. Default: build/.
#                        An existing build-dir/provingKey/ is replaced.
#   --skip-compile-pil   Skip regenerating the *_fixed.bin frops files; reuse
#                        what's on disk. The hash must still match.
#   --force              Skip the input-hash compare. Downloads if the bucket
#                        has the tarball for the current VERSION, regardless
#                        of local inputs. Use when you just want the latest
#                        published key without paying for cargo + frops.
#   --release            Look up the release-namespace bucket artifact
#                        (zisk-provingkey-<VERSION>.*). Without this flag,
#                        the "pre-<VERSION>" namespace is used, matching the
#                        default upload namespace of package-proving-key.sh.
#
# Exit codes:
#   0  cache hit, extracted to <build-dir>/provingKey/
#   1  argument error / missing tool / environment problem
#   2  cache miss (no remote sidecar, hash mismatch, or --force with no tarball)

set -euo pipefail

BUILD_DIR="build"
SKIP_COMPILE_PIL=0
FORCE=0
RELEASE=0

usage() {
  sed -n '2,/^set -euo/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --build-dir)         BUILD_DIR="$2";       shift 2 ;;
    --skip-compile-pil)  SKIP_COMPILE_PIL=1;   shift ;;
    --force)             FORCE=1;              shift ;;
    --release)           RELEASE=1;            shift ;;
    -h|--help)           usage ;;
    *) echo "unknown arg: $1" >&2; usage ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

command -v jq   >/dev/null || { echo "jq not on PATH" >&2; exit 1; }
command -v curl >/dev/null || { echo "curl not on PATH" >&2; exit 1; }

# Sets PROOFMAN_DIR / VERSION / BUCKET / PK_NAME / HASH_NAME / INCLUDE_PATHS
# and defines generate_frops + compute_input_hash. See lib/setup-common.sh.
. "$SCRIPT_DIR/lib/setup-common.sh"

echo "version: $VERSION"

if [ $FORCE -eq 0 ]; then
  generate_frops
  LOCAL_HASH="$(compute_input_hash)"
  echo "local input hash: $LOCAL_HASH"

  remote_hash_tmp=$(mktemp)
  remote_hash=""
  if curl -fsSL "${BUCKET}/${HASH_NAME}" -o "$remote_hash_tmp" 2>/dev/null; then
    remote_hash="$(awk '{print $1}' "$remote_hash_tmp")"
  fi
  rm -f "$remote_hash_tmp"

  if [ -z "$remote_hash" ]; then
    echo "cache miss — no remote hash sidecar at ${BUCKET}/${HASH_NAME}" >&2
    exit 2
  fi
  if [ "$remote_hash" != "$LOCAL_HASH" ]; then
    echo "cache miss — remote hash ${remote_hash} != local ${LOCAL_HASH}" >&2
    exit 2
  fi
  echo "cache hit — remote hash matches"
else
  echo "--force: skipping hash compare; will download whatever is at ${BUCKET}/${PK_NAME}"
fi

mkdir -p "$BUILD_DIR"
tarball="$(mktemp_tarball)"
trap 'rm -f "$tarball"' EXIT

echo "==> downloading ${PK_NAME}"
if ! curl -fL --progress-bar "${BUCKET}/${PK_NAME}" -o "$tarball"; then
  echo "download failed — no tarball at ${BUCKET}/${PK_NAME}" >&2
  exit 2
fi

echo "==> extracting into $BUILD_DIR/"
rm -rf "$BUILD_DIR/provingKey"
tar -xzf "$tarball" -C "$BUILD_DIR"
echo "done — provingKey extracted to $BUILD_DIR/provingKey"
