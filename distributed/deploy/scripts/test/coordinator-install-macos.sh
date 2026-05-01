#!/usr/bin/env bash
# coordinator-install-macos.sh — end-to-end smoke for the coordinator install on a
# real macOS host. Mirror of worker-install-macos.sh with coordinator-specific paths.
#
# Usage (run on macOS, as root):
#   cd <zisk-clone>
#   sudo bash distributed/deploy/scripts/test/coordinator-install-macos.sh           # safe default
#   sudo bash distributed/deploy/scripts/test/coordinator-install-macos.sh --load    # also launchctl load
#
# Differences from worker:
#   - Coordinator unit/plist sets NO env vars (Stage 1 dropped the HOME=
#     workaround; coordinator code doesn't read $HOME/.zisk paths).
#   - No cache/ or inputs/ subdirs under WORK_DIR (coordinator doesn't run
#     rom-setup).
#   - Bundle population is a near-no-op if the worker installed first (idempotent).
#   - --api-port flag is required-ish (default 7000); we pass the default.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

LOAD_MODE=false
for arg in "$@"; do
    case "$arg" in
        --load) LOAD_MODE=true ;;
        -h|--help)
            awk 'NR==1 {next} /^#/ {print; next} {exit}' "$0"
            exit 0
            ;;
    esac
done

COORD_INSTALL="$(cd "${SCRIPT_DIR}/../coordinator" && pwd)/install.sh"
[[ -x "$COORD_INSTALL" ]] || { echo "coordinator/install.sh not found at $COORD_INSTALL"; exit 1; }

BUNDLE='/Library/Application Support/ZisK'
PLIST='/Library/LaunchDaemons/com.zisk.coordinator.plist'
NEWSYSLOG='/etc/newsyslog.d/zisk-coordinator.conf'
WORK_DIR='/usr/local/var/zisk-coordinator'
LOG_DIR='/var/log/zisk-coordinator'
SVC_BIN='/usr/local/bin/zisk-coordinator'

# ── pre-flight ────────────────────────────────────────────────────────────────
info "Pre-flight"
[[ "$(uname -s)" == "Darwin" ]] && ok "host is Darwin" || { fail "macOS only"; exit 1; }
[[ "$EUID" -eq 0 ]] && ok "running as root" || { fail "must run as sudo / root"; exit 1; }

check_github_reachable
check_tarball_exists darwin

if launchctl print system/com.zisk.coordinator >/dev/null 2>&1; then
    fail "com.zisk.coordinator is already loaded — refusing to clobber. Run 'sudo $COORD_INSTALL --uninstall' first."
    exit 1
fi
if [[ -f "$PLIST" ]]; then
    fail "$PLIST already exists — refusing to clobber. Run 'sudo $COORD_INSTALL --uninstall' first."
    exit 1
fi

# ── 1. install ────────────────────────────────────────────────────────────────
info "Running coordinator install.sh (--no-enable -y)"
"$COORD_INSTALL" --no-enable -y 2>&1 | sed 's/^/    /' || \
    { fail "coordinator install.sh exited non-zero"; exit 1; }

# ── 2. bundle layout (shared with worker; coordinator triggers ziskup) ────────
info "Bundle layout under $BUNDLE"
[[ -d "$BUNDLE/bin" ]] && ok "$BUNDLE/bin exists" || fail "$BUNDLE/bin missing"
[[ -f "$BUNDLE/bin/zisk-coordinator" ]] && ok "bundle has zisk-coordinator" \
    || fail "bundle missing zisk-coordinator"
[[ -f "$BUNDLE/bin/ziskup" ]] && ok "bundle has ziskup" || fail "bundle missing ziskup"

# ── 3. ownership + perms ──────────────────────────────────────────────────────
info "Bundle ownership + perms"
got=$(stat -f '%Su:%Sg' "$BUNDLE/bin/zisk-coordinator")
[[ "$got" == "zisk:zisk" ]] && ok "owner zisk:zisk on zisk-coordinator" || fail "owner wrong: $got"
got=$(stat -f '%Lp' "$BUNDLE")
[[ "$got" == "750" ]] && ok "$BUNDLE is 0750" || fail "$BUNDLE perm $got"

# ── 4. dscl users + groups ────────────────────────────────────────────────────
info "dscl users + groups"
dscl . -read /Groups/zisk             >/dev/null 2>&1 && ok "group 'zisk' exists" \
    || fail "group 'zisk' missing"
dscl . -read /Users/zisk              >/dev/null 2>&1 && ok "user 'zisk' exists" \
    || fail "user 'zisk' missing"
dscl . -read /Groups/zisk-coordinator >/dev/null 2>&1 && ok "group 'zisk-coordinator' exists" \
    || fail "group 'zisk-coordinator' missing"
dscl . -read /Users/zisk-coordinator  >/dev/null 2>&1 && ok "user 'zisk-coordinator' exists" \
    || fail "user 'zisk-coordinator' missing"
if dseditgroup -o checkmember -m zisk-coordinator zisk >/dev/null 2>&1; then
    ok "zisk-coordinator is a member of zisk group"
else
    fail "zisk-coordinator NOT a member of zisk group"
fi

# ── 5. per-service state ──────────────────────────────────────────────────────
info "Per-service state under $WORK_DIR"
[[ -d "$WORK_DIR" ]] && ok "$WORK_DIR exists" || fail "$WORK_DIR missing"
# Coordinator does NOT need cache/ or inputs/.
[[ ! -d "$WORK_DIR/cache"  ]] && ok "$WORK_DIR/cache absent (correct)"  \
    || warn "$WORK_DIR/cache present — unexpected"
[[ ! -d "$WORK_DIR/inputs" ]] && ok "$WORK_DIR/inputs absent (correct)" \
    || warn "$WORK_DIR/inputs present — unexpected"
# No legacy .zisk/ wrapper.
[[ ! -d "$WORK_DIR/.zisk" ]] && ok "$WORK_DIR/.zisk absent (no legacy wrapper)" \
    || fail "legacy $WORK_DIR/.zisk dir present"

# ── 6. service binary at /usr/local/bin ───────────────────────────────────────
info "Service binary"
[[ -f "$SVC_BIN" && -x "$SVC_BIN" ]] && ok "$SVC_BIN present + executable" \
    || fail "$SVC_BIN missing or not executable"

# ── 7. plist ──────────────────────────────────────────────────────────────────
info "launchd plist at $PLIST"
[[ -f "$PLIST" ]] && ok "plist exists" || { fail "plist missing"; exit 1; }
if command -v xmllint >/dev/null 2>&1 && xmllint --noout "$PLIST" 2>/dev/null; then
    ok "plist is valid XML"
else
    if command -v xmllint >/dev/null 2>&1; then
        fail "plist failed xmllint"
        xmllint --noout "$PLIST" 2>&1 | head -5
    fi
fi
if plutil -lint "$PLIST" >/dev/null 2>&1; then
    ok "plist passes plutil -lint"
else
    fail "plutil rejects plist"
    plutil -lint "$PLIST" || true
fi

# Coordinator unit/plist must NOT set any env vars (Stage 1 cleanup).
if grep -q '<key>EnvironmentVariables</key>' "$PLIST"; then
    fail "plist still has EnvironmentVariables (Stage 1 should have removed it)"
else
    ok "plist has NO EnvironmentVariables block (correct)"
fi
if grep -E '<key>\s*HOME\s*</key>' "$PLIST" >/dev/null; then
    fail "plist still sets HOME"
else
    ok "plist does NOT set HOME"
fi

# WorkingDirectory should still point at WORK_DIR.
grep -A1 '<key>WorkingDirectory</key>' "$PLIST" \
    | grep -q "<string>${WORK_DIR}</string>" \
    && ok "plist WorkingDirectory = $WORK_DIR" \
    || fail "plist WorkingDirectory wrong"

# --api-port should default to 7000 in the plist.
grep -A1 '<string>--api-port</string>' "$PLIST" | grep -q '<string>7000</string>' \
    && ok "plist --api-port = 7000 (default)" \
    || fail "plist --api-port not 7000"

# ── 8. newsyslog rotation ─────────────────────────────────────────────────────
info "newsyslog config"
[[ -f "$NEWSYSLOG" ]] && ok "newsyslog config at $NEWSYSLOG" || fail "newsyslog config missing"

# ── 9. optional: launchctl load ───────────────────────────────────────────────
if $LOAD_MODE; then
    info "Loading daemon via launchctl (--load mode)"
    launchctl load -w "$PLIST"
    sleep 1
    if launchctl print system/com.zisk.coordinator >/dev/null 2>&1; then
        ok "launchctl print recognizes the unit"
    else
        fail "launchctl print doesn't see the unit"
    fi
    # Verify the WorkingDirectory and program args via launchctl print.
    if launchctl print system/com.zisk.coordinator 2>/dev/null | grep -q "working directory = $WORK_DIR"; then
        ok "launchd reports correct WorkingDirectory"
    else
        warn "could not confirm WorkingDirectory via launchctl print (output format varies by macOS version)"
    fi
    launchctl unload "$PLIST" 2>/dev/null || true
    ok "unloaded after verification"
else
    warn "Skipping launchctl load (default mode). Re-run with --load for full verification."
fi

# ── 10. uninstall sweep ───────────────────────────────────────────────────────
info "Running coordinator install.sh --uninstall -y"
"$COORD_INSTALL" --uninstall -y 2>&1 | sed 's/^/    /' || warn "uninstall exited non-zero"

[[ ! -f "$PLIST"     ]] && ok "plist removed"            || fail "plist still present"
[[ ! -f "$SVC_BIN"   ]] && ok "service binary removed"   || fail "$SVC_BIN still present"
[[ ! -f "$NEWSYSLOG" ]] && ok "newsyslog config removed" || fail "newsyslog still present"
[[ ! -d "$WORK_DIR"  ]] && ok "$WORK_DIR removed" \
    || warn "$WORK_DIR still present (uninstall may have skipped)"
if dscl . -read /Users/zisk-coordinator >/dev/null 2>&1; then
    warn "zisk-coordinator user still exists after uninstall"
else
    ok "zisk-coordinator user removed"
fi
# Shared 'zisk' user/group + bundle preserved intentionally.
if dscl . -read /Groups/zisk >/dev/null 2>&1; then
    ok "shared 'zisk' group preserved (other services may use it)"
fi
if [[ -d "$BUNDLE" ]]; then
    ok "$BUNDLE preserved (intentional — shared toolchain payload)"
fi

# ── summary ───────────────────────────────────────────────────────────────────
echo
if [[ "$FAIL" -eq 0 ]]; then
    printf "\033[32m== Total: %d passed, %d failed ==\033[0m\n" "$PASS" "$FAIL"
    echo
    echo "Bundle was preserved at $BUNDLE (uninstall keeps the shared toolchain)."
    echo "To remove the bundle entirely:"
    echo "    sudo rm -rf '$BUNDLE'"
    echo "    sudo dscl . -delete /Users/zisk"
    echo "    sudo dscl . -delete /Groups/zisk"
    exit 0
else
    printf "\033[31m== Total: %d passed, %d failed ==\033[0m\n" "$PASS" "$FAIL"
    echo
    echo "Cleanup hint:"
    echo "    sudo $COORD_INSTALL --uninstall -y"
    echo "    sudo launchctl unload '$PLIST' 2>/dev/null"
    echo "    sudo rm -f '$PLIST' '$NEWSYSLOG' '$SVC_BIN'"
    echo "    sudo rm -rf '$BUNDLE' '$WORK_DIR' '$LOG_DIR'"
    echo "    sudo dscl . -delete /Users/zisk-coordinator"
    echo "    sudo dscl . -delete /Groups/zisk-coordinator"
    echo "    sudo dscl . -delete /Users/zisk"
    echo "    sudo dscl . -delete /Groups/zisk"
    exit 1
fi
