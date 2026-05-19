#!/usr/bin/env bash
#
# Package and upload Zisk proving-key artifacts to gs://zisk-setup/.
#
# Layout (relative to --build-dir):
#   provingKey/        -> zisk-provingkey-<VERSION>.tar.gz  (+ .md5 sidecar)
#   circom/            -> zisk-circuits-<VERSION>.tar.gz
#   provingKeySnark/   -> zisk-provingkey-plonk-<VERSION>.tar.gz
#
# In default and --all modes, if <build-dir>/.input-hash exists (written by
# build-setup.sh on a fresh cache-miss build), it is also uploaded as
# zisk-provingkey-<VERSION>.input-hash so the next build-setup.sh run can hit
# cache. If the file is missing, the sidecar upload is skipped — no error.
#
# <VERSION> is read from the workspace `[workspace.package].version` in the
# repo's root Cargo.toml.
#
# Modes:
#   (default)   package provingKey + circuits (and hash sidecar if present)
#   --snark     package provingKeySnark only (run after `proofman-setup setup-snark`)
#   --all       package provingKey + circuits + snark (and hash sidecar if present)
#
# All produced files are pushed to gs://zisk-setup/ via `gcloud storage`.

set -euo pipefail

BUCKET="gs://zisk-setup"

usage() {
  cat <<EOF >&2
usage: $0 --build-dir DIR [--snark | --all] [--out-dir DIR]

  --build-dir DIR    Directory produced by \`cargo zisk proofman-setup setup\`.
  --snark            Package only the snark output (provingKeySnark/).
  --all              Package proving key, circuits, and snark.
  --out-dir DIR      Where to write tarballs. Default: <repo root>/dist

Artifacts are always uploaded to ${BUCKET}/ via gcloud storage.
EOF
  exit 1
}

BUILD_DIR=""
MODE="standard"   # standard | snark | all
OUT_DIR=""

while [ $# -gt 0 ]; do
  case "$1" in
    --build-dir)
      BUILD_DIR="$2"
      shift 2
      ;;
    --snark)
      MODE="snark"
      shift
      ;;
    --all)
      MODE="all"
      shift
      ;;
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    -h|--help)
      usage
      ;;
    *)
      echo "unknown arg: $1" >&2
      usage
      ;;
  esac
done

[ -n "$BUILD_DIR" ] || usage
[ -d "$BUILD_DIR" ] || {
  echo "build dir not found: $BUILD_DIR" >&2
  exit 1
}

command -v gcloud >/dev/null || {
  echo "gcloud not found in PATH" >&2
  exit 1
}

# Fail fast on missing GCS auth before we spend time tarring multi-GB artifacts.
gcloud storage ls "${BUCKET}/" >/dev/null

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Workspace version: first `version = "..."` line in root Cargo.toml.
VERSION="$(awk -F'"' '/^version[[:space:]]*=/ { print $2; exit }' "$ROOT_DIR/Cargo.toml")"

[ -n "$VERSION" ] || {
  echo "could not read workspace version from $ROOT_DIR/Cargo.toml" >&2
  exit 1
}

echo "version: $VERSION"

OUT_DIR="${OUT_DIR:-$ROOT_DIR/dist}"
mkdir -p "$OUT_DIR"

ARTIFACTS=()

write_md5() {
  local file="$1"

  if command -v md5sum >/dev/null 2>&1; then
    md5sum "$file"
  elif command -v md5 >/dev/null 2>&1; then
    md5 -r "$file"
  else
    echo "no md5 utility found (need md5sum or md5)" >&2
    exit 1
  fi
}

pack() {
  local src="$1"
  local tarname="$2"
  local with_md5="$3"

  if [ ! -d "$BUILD_DIR/$src" ]; then
    echo "skipping $src (not found in $BUILD_DIR)" >&2
    return
  fi

  local tarball="$OUT_DIR/$tarname"

  echo "packing $BUILD_DIR/$src -> $tarball"

  tar -czf "$tarball" -C "$BUILD_DIR" "$src"

  ARTIFACTS+=("$tarball")

  if [ "$with_md5" = "yes" ]; then
    (
      cd "$OUT_DIR"
      write_md5 "$tarname" > "$tarname.md5"
    )

    ARTIFACTS+=("$tarball.md5")
  fi
}

case "$MODE" in
  standard|all)
    pack provingKey      "zisk-provingkey-${VERSION}.tar.gz"       yes
    pack circom          "zisk-circuits-${VERSION}.tar.gz"         no
    ;;
esac

case "$MODE" in
  snark|all)
    pack provingKeySnark "zisk-provingkey-plonk-${VERSION}.tar.gz" no
    ;;
esac

# Hash sidecar — only meaningful for the (recursive) provingKey publish.
# build-setup.sh drops it on cache-miss runs; absence is normal for --snark or
# --skip-compile-pil paths and just means the cache won't get refreshed.
case "$MODE" in
  standard|all)
    if [ -f "$BUILD_DIR/.input-hash" ]; then
      hash_copy="$OUT_DIR/zisk-provingkey-${VERSION}.input-hash"

      cp "$BUILD_DIR/.input-hash" "$hash_copy"

      ARTIFACTS+=("$hash_copy")

      echo "including hash sidecar: $hash_copy"
    else
      echo "no $BUILD_DIR/.input-hash — skipping hash sidecar (cache will not be refreshed)" >&2
    fi
    ;;
esac

if [ ${#ARTIFACTS[@]} -eq 0 ]; then
  echo "no artifacts produced" >&2
  exit 1
fi

echo "uploading ${#ARTIFACTS[@]} file(s) to ${BUCKET}/"

gcloud storage cp "${ARTIFACTS[@]}" "${BUCKET}/"

echo "done. artifacts:"

printf '  %s\n' "${ARTIFACTS[@]}"