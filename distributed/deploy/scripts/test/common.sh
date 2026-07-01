#!/usr/bin/env bash
# common.sh — shared helpers for install-smoke scripts. Sourced by
# worker-install-{linux,macos}.sh and coordinator-install-{linux,macos}.sh.

# Counters — sourcing script reads these in its summary block.
PASS=${PASS:-0}
FAIL=${FAIL:-0}

ok()   { printf "  \033[32m✓\033[0m %s\n" "$*"; PASS=$((PASS+1)); }
fail() { printf "  \033[31m✗\033[0m %s\n" "$*" >&2; FAIL=$((FAIL+1)); }
info() { printf "\033[1;36m== %s ==\033[0m\n" "$*"; }
warn() { printf "\033[1;33m! %s\033[0m\n" "$*" >&2; }

# Echoes arm64 or amd64 based on uname -m (default amd64 for unknowns).
resolve_arch() {
    case "$(uname -m)" in
        arm64|aarch64) echo arm64 ;;
        x86_64)        echo amd64 ;;
        *)             echo amd64 ;;
    esac
}

# Asserts github.com is reachable. Calls fail+exit on error.
check_github_reachable() {
    if curl -fsI -o /dev/null --max-time 5 https://github.com/0xPolygonHermez/zisk/releases/latest 2>/dev/null; then
        ok "github.com reachable"
    else
        fail "github.com unreachable — ziskup --system needs to download release tarball"
        exit 1
    fi
}

# check_tarball_exists OS_NAME
# Verifies the release tarball for ${OS_NAME}_$(arch).tar.gz exists at GitHub.
# Catches the missing-arch case (e.g., macOS amd64 not always shipped) before
# ziskup runs and produces a confusing tar/gzip error from a 404 HTML page.
check_tarball_exists() {
    local os_name="$1"
    local arch
    arch="$(resolve_arch)"
    local url="https://github.com/0xPolygonHermez/zisk/releases/latest/download/cargo_zisk_${os_name}_${arch}.tar.gz"
    if curl -fsIL -o /dev/null --max-time 10 "$url" 2>/dev/null; then
        ok "release tarball exists for ${os_name}_${arch}"
    else
        fail "release tarball NOT found at $url"
        fail "the latest release may not ship this OS/arch combination."
        exit 1
    fi
}
