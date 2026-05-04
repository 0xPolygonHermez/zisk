#!/usr/bin/env bash
#
# Package and upload Zisk proving-key artifacts to gs://zisk-setup/.
#
# Layout (relative to --build-dir):
#   provingKey/        -> zisk-provingkey-<VERSION>.tar.gz  (+ .md5 sidecar)
#   circom/            -> zisk-circuits-<VERSION>.tar.gz
#   provingKeySnark/   -> zisk-provingkey-snark-<VERSION>.tar.gz
#
# <VERSION> is read from the workspace `[workspace.package].version` in the
# repo's root Cargo.toml.
#
# Modes:
#   (default)   package provingKey + circuits
#   --snark     package provingKeySnark only (run after `proofman-setup setup-snark`)
#   --all       package provingKey + circuits + snark
#
# All produced files are pushed to gs://zisk-setup/ via `gsutil`.

set -euo pipefail

BUCKET="gs://zisk-setup"

usage() {
  cat <<EOF >&2
usage: $0 --build-dir DIR [--snark | --all] [--out-dir DIR]

  --build-dir DIR    Directory produced by \`cargo zisk proofman-setup setup\`.
  --snark            Package only the snark output (provingKeySnark/).
  --all              Package proving key, circuits, and snark.
  --out-dir DIR      Where to write tarballs. Default: <repo root>/dist

Artifacts are always uploaded to ${BUCKET}/ via gsutil.
EOF
  exit 1
}

BUILD_DIR=""
MODE="standard"   # standard | snark | all
OUT_DIR=""

while [ $# -gt 0 ]; do
  case "$1" in
    --build-dir) BUILD_DIR="$2"; shift 2 ;;
    --snark)     MODE="snark";   shift ;;
    --all)       MODE="all";     shift ;;
    --out-dir)   OUT_DIR="$2";   shift 2 ;;
    -h|--help)   usage ;;
    *) echo "unknown arg: $1" >&2; usage ;;
  esac
done

[ -n "$BUILD_DIR" ] || usage
[ -d "$BUILD_DIR" ] || { echo "build dir not found: $BUILD_DIR" >&2; exit 1; }
command -v gsutil >/dev/null || { echo "gsutil not found in PATH" >&2; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Workspace version: first `version = "..."` line in root Cargo.toml.
VERSION="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$ROOT_DIR/Cargo.toml")"
[ -n "$VERSION" ] || { echo "could not read workspace version from $ROOT_DIR/Cargo.toml" >&2; exit 1; }
echo "version: $VERSION"

OUT_DIR="${OUT_DIR:-$ROOT_DIR/dist}"
mkdir -p "$OUT_DIR"

ARTIFACTS=()

pack() {
  local src="$1" tarname="$2" with_md5="$3"
  if [ ! -d "$BUILD_DIR/$src" ]; then
    echo "skipping $src (not found in $BUILD_DIR)" >&2
    return
  fi
  local tarball="$OUT_DIR/$tarname"
  echo "packing $BUILD_DIR/$src -> $tarball"
  tar -czf "$tarball" -C "$BUILD_DIR" "$src"
  ARTIFACTS+=("$tarball")
  if [ "$with_md5" = "yes" ]; then
    (cd "$OUT_DIR" && md5sum "$tarname" > "$tarname.md5")
    ARTIFACTS+=("$tarball.md5")
  fi
}

case "$MODE" in
  standard|all)
    pack provingKey       "zisk-provingkey-${VERSION}.tar.gz"        yes
    pack circom           "zisk-circuits-${VERSION}.tar.gz"          no
    ;;
esac
case "$MODE" in
  snark|all)
    pack provingKeySnark  "zisk-provingkey-snark-${VERSION}.tar.gz"  no
    ;;
esac

if [ ${#ARTIFACTS[@]} -eq 0 ]; then
  echo "no artifacts produced" >&2
  exit 1
fi

echo "uploading ${#ARTIFACTS[@]} file(s) to ${BUCKET}/"
gsutil -m cp "${ARTIFACTS[@]}" "${BUCKET}/"

echo "done. artifacts:"
printf '  %s\n' "${ARTIFACTS[@]}"
