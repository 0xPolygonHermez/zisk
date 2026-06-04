#!/bin/bash
#
# Fetch / check a published ZisK setup from the public bucket.
#
# Subcommands (default = fetch):
#   (default)         Download + install the latest setup into $HOME/.zisk and
#                     record $HOME/.zisk/provingKey/.setup-hash (the published
#                     input hash from the bucket .hash sidecar).
#   --check input     Compute the local build-input hash (build-setup.sh
#                     --print-hash) and compare to the bucket .hash.
#                     Prints: new | unchanged   (with --force => always new).
#                     Prints local_hash=<hex> to stderr.
#   --check installed Compare $HOME/.zisk/provingKey/.setup-hash to the bucket
#                     .hash. Prints: current | outdated | unknown.
#
# Options:
#   --version V   Setup version/name (default: Cargo.toml gha_package_setup_version).
#   --force       (with --check input) always print "new".
#
# Reads use https://storage.googleapis.com/zisk-setup/ (public); the .hash
# sidecar is read via `gcloud storage cat gs://zisk-setup/...` so it also works
# before a setup is public. gcloud is only required for the .hash read.
set -euo pipefail

source ./utils.sh

BUCKET_HTTP="https://storage.googleapis.com/zisk-setup"
BUCKET_GS="gs://zisk-setup"
MODE="fetch"     # fetch | check-input | check-installed
VERSION=""
FORCE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --check)
      case "${2:-}" in
        input)     MODE="check-input" ;;
        installed) MODE="check-installed" ;;
        *) echo "error: --check needs 'input' or 'installed'" >&2; exit 2 ;;
      esac
      shift 2 ;;
    --version) VERSION="$2"; shift 2 ;;
    --force)   FORCE=1; shift ;;
    -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

# Resolve the version (input wins; else Cargo.toml gha_package_setup_version).
resolve_version() {
  if [ -z "$VERSION" ]; then
    VERSION="$(get_var_from_cargo_toml PACKAGE_SETUP_VERSION)"
  fi
  [ -n "$VERSION" ] || { err "could not resolve setup version (pass --version or set gha_package_setup_version in Cargo.toml)" true; exit 1; }
}

remote_hash() {
  # Echo the bucket .hash contents (empty if absent). Requires gcloud.
  command -v gcloud >/dev/null || { err "gcloud not on PATH (needed to read the .hash sidecar)" true; exit 1; }
  gcloud storage cat "$BUCKET_GS/zisk-provingkey-${VERSION}.hash" 2>/dev/null | tr -d '[:space:]' || true
}

case "$MODE" in
  check-input)
    resolve_version
    LOCAL_HASH="${LOCAL_HASH:-$("$(dirname "$0")/../setup/build-setup.sh" --print-hash 2>/dev/null | tail -n 1)}"
    [ -n "$LOCAL_HASH" ] || { err "empty local hash" true; exit 1; }
    echo "local_hash=$LOCAL_HASH" >&2
    if [ "$FORCE" -eq 1 ]; then echo "new"; exit 0; fi
    REMOTE="$(remote_hash)"
    if [ -n "$REMOTE" ] && [ "$REMOTE" = "$LOCAL_HASH" ]; then echo "unchanged"; else echo "new"; fi
    ;;

  check-installed)
    resolve_version
    MARKER="$HOME/.zisk/provingKey/.setup-hash"
    if [ ! -f "$MARKER" ]; then echo "unknown"; exit 0; fi
    INSTALLED="$(tr -d '[:space:]' < "$MARKER")"
    REMOTE="$(remote_hash)"
    if [ -z "$REMOTE" ]; then echo "unknown"; exit 0; fi
    if [ "$INSTALLED" = "$REMOTE" ]; then echo "current"; else echo "outdated"; fi
    ;;

  fetch)
    resolve_version
    FILE="zisk-provingkey-${VERSION}.tar.gz"
    info "Fetching ${FILE}..."
    ensure curl -L -#o "${FILE}" "${BUCKET_HTTP}/${FILE}"
    info "Installing into \$HOME/.zisk..."
    ensure rm -rf "$HOME/.zisk/provingKey/"
    ensure mkdir -p "$HOME/.zisk"
    ensure tar -xf "${FILE}" -C "$HOME/.zisk"
    rm -f "${FILE}"
    # Record the published hash as the install marker (best-effort; needs gcloud).
    REMOTE="$(remote_hash || true)"
    if [ -n "$REMOTE" ]; then
      printf '%s' "$REMOTE" > "$HOME/.zisk/provingKey/.setup-hash"
      info "Recorded install marker .setup-hash=${REMOTE}"
    else
      warn "No .hash sidecar for ${VERSION}; --check installed will report 'unknown'."
    fi
    success "Setup ${VERSION} installed. Run 'cargo-zisk check-setup' to generate const trees."
    ;;
esac
