#!/usr/bin/env bash
# worker-install-macos.sh — end-to-end smoke for the worker install on a real macOS host.
#
# Usage (run on macOS, as root):
#   cd <zisk-clone>
#   sudo bash distributed/deploy/scripts/test/worker-install-macos.sh           # safe default
#   sudo bash distributed/deploy/scripts/test/worker-install-macos.sh --load    # also launchctl load
#
# What it does:
#   1. Pre-flight: Darwin, root, internet
#   2. Runs distributed/deploy/scripts/worker/install.sh --no-mpi --no-enable -y
#      (exercises real ziskup --system download, dscl user/group creation,
#       /Library/Application Support/ZisK bundle layout, plist write).
#   3. Asserts filesystem layout, ownership, perms, plist XML validity & content,
#      dscl user/group existence, supplementary group membership.
#   4. With --load: also launchctl load -w and verify the unit is loaded with
#      the expected Environment vars. (Without --load, plist is written but not
#      loaded into launchd — safe default.)
#   5. Sweeps state via worker/install.sh --uninstall -y and verifies cleanup.
#
# Safety:
#   - Default mode writes files + creates dscl users + writes plist BUT does not
#     load the launchd daemon. Uninstall step at the end reverses everything.
#   - --load mode loads the daemon; worker will try to start, fail without a
#     coordinator, and Restart=on-failure will loop. We unload before uninstall.
#   - Test requires sudo. It modifies system state (dscl, /Library, /etc, /var).
#     All changes are reversed by the uninstall step at the end.

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

WORKER_INSTALL="$(cd "${SCRIPT_DIR}/../worker" && pwd)/install.sh"
[[ -x "$WORKER_INSTALL" ]] || { echo "worker/install.sh not found at $WORKER_INSTALL"; exit 1; }

BUNDLE='/Library/Application Support/ZisK'
PLIST='/Library/LaunchDaemons/com.zisk.worker.plist'
NEWSYSLOG='/etc/newsyslog.d/zisk-worker.conf'
# macOS state path — defaults.env now uses /usr/local/var/ on Darwin (avoids
# SIP issues with /var/lib/ that the previous test exposed).
WORK_DIR='/usr/local/var/zisk-worker'
LOG_DIR='/var/log/zisk-worker'
SVC_BIN='/usr/local/bin/zisk-worker'

# ── pre-flight ────────────────────────────────────────────────────────────────
info "Pre-flight"
[[ "$(uname -s)" == "Darwin" ]] && ok "host is Darwin" || { fail "this test is for macOS only"; exit 1; }
[[ "$EUID" -eq 0 ]] && ok "running as root" || { fail "must run as sudo / root"; exit 1; }

check_github_reachable
# tarball check catches the missing-arch case (e.g., Intel Mac when only
# darwin_arm64 is shipped) — would otherwise 404 and tar would fail with
# a confusing 'not in gzip format' error from the saved HTML page.
check_tarball_exists darwin

# Detect previous install; bail to avoid clobbering operator's real deploy.
if launchctl print system/com.zisk.worker >/dev/null 2>&1; then
    fail "com.zisk.worker is already loaded — refusing to clobber a real deploy. Run 'sudo $WORKER_INSTALL --uninstall' first."
    exit 1
fi
if [[ -f "$PLIST" ]]; then
    fail "$PLIST already exists — refusing to clobber. Run 'sudo $WORKER_INSTALL --uninstall' first."
    exit 1
fi

# ── 1. install ────────────────────────────────────────────────────────────────
info "Running worker install.sh (--no-mpi --no-enable -y)"
"$WORKER_INSTALL" --no-mpi --no-enable -y 2>&1 | sed 's/^/    /' || \
    { fail "worker install.sh exited non-zero"; exit 1; }

# ── 2. bundle layout ──────────────────────────────────────────────────────────
info "Bundle layout under $BUNDLE"
[[ -d "$BUNDLE/bin" ]] && ok "$BUNDLE/bin exists" || fail "$BUNDLE/bin missing"
for f in cargo-zisk ziskemu riscv2zisk zisk-worker zisk-coordinator ziskup libziskclib.a; do
    [[ -f "$BUNDLE/bin/$f" ]] && ok "bundle has $f" || fail "missing $BUNDLE/bin/$f"
done
# emulator-asm intentionally NOT in Darwin bundle (asm execution unsupported on macOS).
[[ ! -d "$BUNDLE/zisk/emulator-asm" ]] && ok "no emulator-asm/ on Darwin (correct)" \
    || warn "$BUNDLE/zisk/emulator-asm present — unexpected on macOS"

# ── 3. ownership + perms ──────────────────────────────────────────────────────
info "Bundle ownership + perms"
got=$(stat -f '%Su:%Sg' "$BUNDLE/bin/cargo-zisk")
[[ "$got" == "zisk:zisk" ]] && ok "owner zisk:zisk on cargo-zisk" || fail "bundle owner wrong: got $got"
got=$(stat -f '%Lp' "$BUNDLE")
[[ "$got" == "750" ]] && ok "$BUNDLE is 0750" || fail "$BUNDLE perm $got, expected 750"
got=$(stat -f '%Lp' "$BUNDLE/bin/cargo-zisk")
[[ "$got" == "750" ]] && ok "cargo-zisk is 0750" || fail "cargo-zisk perm $got"

# ── 4. dscl users + groups ────────────────────────────────────────────────────
info "dscl users + groups"
dscl . -read /Groups/zisk >/dev/null 2>&1 && ok "group 'zisk' exists" || fail "group 'zisk' missing"
dscl . -read /Users/zisk  >/dev/null 2>&1 && ok "user 'zisk' exists"  || fail "user 'zisk' missing"
dscl . -read /Groups/zisk-worker >/dev/null 2>&1 && ok "group 'zisk-worker' exists" \
    || fail "group 'zisk-worker' missing"
dscl . -read /Users/zisk-worker  >/dev/null 2>&1 && ok "user 'zisk-worker' exists"  \
    || fail "user 'zisk-worker' missing"
# zisk-worker must be in zisk group (supplementary, so it can read the bundle).
if dseditgroup -o checkmember -m zisk-worker zisk >/dev/null 2>&1; then
    ok "zisk-worker is a member of zisk group"
else
    fail "zisk-worker is NOT a member of zisk group"
fi

# ── 5. per-service state ──────────────────────────────────────────────────────
info "Per-service state under $WORK_DIR"
[[ -d "$WORK_DIR/cache"  ]] && ok "$WORK_DIR/cache exists"  || fail "$WORK_DIR/cache missing"
[[ -d "$WORK_DIR/inputs" ]] && ok "$WORK_DIR/inputs exists" || fail "$WORK_DIR/inputs missing"
[[ ! -d "$WORK_DIR/.zisk" ]] && ok "$WORK_DIR/.zisk absent (no legacy wrapper)" \
    || fail "legacy $WORK_DIR/.zisk dir present"
got=$(stat -f '%Su:%Sg' "$WORK_DIR/cache")
[[ "$got" == "zisk-worker:zisk-worker" ]] && ok "state owner zisk-worker:zisk-worker" \
    || fail "state owner wrong: got $got"

# ── 6. service binary at /usr/local/bin ───────────────────────────────────────
info "Service binary"
[[ -f "$SVC_BIN" ]] && ok "$SVC_BIN exists" || fail "$SVC_BIN missing"
[[ -x "$SVC_BIN" ]] && ok "$SVC_BIN is executable" || fail "$SVC_BIN not executable"

# ── 7. plist (always written; loaded only with --load) ────────────────────────
info "launchd plist at $PLIST"
[[ -f "$PLIST" ]] && ok "plist exists" || { fail "plist missing"; exit 1; }
if command -v xmllint >/dev/null 2>&1; then
    if xmllint --noout "$PLIST" 2>/dev/null; then
        ok "plist is valid XML"
    else
        fail "plist failed XML validation"
        xmllint --noout "$PLIST" 2>&1 | head -5
    fi
fi
# plutil also validates plist format (always available on macOS).
if plutil -lint "$PLIST" >/dev/null 2>&1; then
    ok "plist passes plutil -lint"
else
    fail "plutil rejects plist"
    plutil -lint "$PLIST" || true
fi

# Env-var assertions on the plist.
grep -q '<key>ZISK_HOME</key>' "$PLIST" && ok "plist sets ZISK_HOME" || fail "plist missing ZISK_HOME"
grep -A1 '<key>ZISK_HOME</key>' "$PLIST" \
    | grep -q "<string>${BUNDLE}</string>" \
    && ok "ZISK_HOME = $BUNDLE" \
    || fail "ZISK_HOME points elsewhere"
grep -q '<key>ZISK_CACHE_DIR</key>' "$PLIST" && ok "plist sets ZISK_CACHE_DIR" \
    || fail "plist missing ZISK_CACHE_DIR"
grep -A1 '<key>ZISK_CACHE_DIR</key>' "$PLIST" \
    | grep -q "<string>${WORK_DIR}/cache</string>" \
    && ok "ZISK_CACHE_DIR = $WORK_DIR/cache" \
    || fail "ZISK_CACHE_DIR points elsewhere"
# HOME key must NOT be set (Stage 1 dropped the workaround).
if grep -E '<key>\s*HOME\s*</key>' "$PLIST" >/dev/null; then
    fail "plist still sets HOME (should be removed)"
else
    ok "plist does NOT set HOME"
fi
# proving-key arg should point inside the bundle.
if grep -A1 '<string>--proving-key</string>' "$PLIST" \
   | grep -q "<string>${BUNDLE}/provingKey</string>"; then
    ok "plist --proving-key argument → $BUNDLE/provingKey"
else
    fail "plist --proving-key argument not in bundle"
    grep -B1 -A2 'proving-key' "$PLIST" | head
fi
grep -q "<!-- zisk-worker:CONFIG_FILE=${CONFIG} -->" "$PLIST" \
    && ok "metadata footer has CONFIG_FILE" \
    || fail "metadata footer missing CONFIG_FILE — uninstall would fall back to live global"

# ── 7b. ziskup install receipt ───────────────────────────────────────────────
info "ziskup receipt at $BUNDLE/.zisk-receipt"
RECEIPT="$BUNDLE/.zisk-receipt"
[[ -f "$RECEIPT" ]] && ok "$RECEIPT exists" || { fail "$RECEIPT missing"; exit 1; }
grep -qE '^version=[0-9]+\.[0-9]+\.[0-9]+$' "$RECEIPT" \
    && ok "receipt has version field" \
    || fail "receipt missing/invalid version"
grep -qE '^manifest=.*\bbin\b' "$RECEIPT" \
    && ok "receipt manifest includes 'bin'" \
    || fail "receipt manifest missing 'bin'"
grep -q '^created_user=zisk$' "$RECEIPT" \
    && ok "receipt records created_user=zisk" \
    || fail "receipt missing created_user (ziskup --uninstall would skip user removal)"
grep -q '^created_group=zisk$' "$RECEIPT" \
    && ok "receipt records created_group=zisk" \
    || fail "receipt missing created_group"

# ── 8. newsyslog rotation config ──────────────────────────────────────────────
info "newsyslog config"
[[ -f "$NEWSYSLOG" ]] && ok "newsyslog config at $NEWSYSLOG" || fail "newsyslog config missing"

# ── 9. optional: actually load the daemon and verify env vars are live ────────
if $LOAD_MODE; then
    info "Loading daemon via launchctl (--load mode)"
    launchctl load -w "$PLIST"
    sleep 1
    if launchctl print system/com.zisk.worker >/dev/null 2>&1; then
        ok "launchctl print recognizes the unit"
    else
        fail "launchctl print doesn't see the unit"
    fi
    env_dump=$(launchctl print system/com.zisk.worker 2>/dev/null | awk '/environment/,/^[^[:space:]]/' || true)
    echo "$env_dump" | grep -q "ZISK_HOME = $BUNDLE" \
        && ok "launchd env has ZISK_HOME = $BUNDLE" \
        || fail "launchd env missing ZISK_HOME"
    echo "$env_dump" | grep -q "ZISK_CACHE_DIR = $WORK_DIR/cache" \
        && ok "launchd env has ZISK_CACHE_DIR" \
        || fail "launchd env missing ZISK_CACHE_DIR"

    # The actual zisk-worker process will fail (no coordinator). That's fine —
    # we only care that launchd loaded the unit with the right env. Unload.
    launchctl unload "$PLIST" 2>/dev/null || true
    ok "unloaded after verification"
else
    warn "Skipping launchctl load (default mode). Re-run with --load for full verification."
fi

# ── 10. uninstall sweep ───────────────────────────────────────────────────────
info "Running worker install.sh --uninstall -y"
UNINSTALL_OUT=$("$WORKER_INSTALL" --uninstall -y 2>&1) || warn "uninstall exited non-zero"
echo "$UNINSTALL_OUT" | sed 's/^/    /'
echo "$UNINSTALL_OUT" | grep -q 'sudo ziskup --uninstall --system' \
    && ok "uninstall output points operator at 'ziskup --uninstall --system'" \
    || fail "uninstall output missing bundle-removal hint"

[[ ! -f "$PLIST"      ]] && ok "plist removed"            || fail "plist still present"
[[ ! -f "$SVC_BIN"    ]] && ok "service binary removed"   || fail "$SVC_BIN still present"
[[ ! -f "$NEWSYSLOG"  ]] && ok "newsyslog config removed" || fail "newsyslog still present"
# Uninstall prompts for dirs + svc user removal; with -y it should remove them.
[[ ! -d "$WORK_DIR" ]] && ok "$WORK_DIR removed"          || warn "$WORK_DIR still present (may be intentional if uninstall declined)"
if dscl . -read /Users/zisk-worker >/dev/null 2>&1; then
    warn "zisk-worker user still exists after uninstall (uninstall may have skipped user removal)"
else
    ok "zisk-worker user removed"
fi
# Note: the shared 'zisk' group/user is intentionally NOT removed by uninstall
# (other services may use it). Do not assert its absence.
if dscl . -read /Groups/zisk >/dev/null 2>&1; then
    ok "shared 'zisk' group preserved (other services may use it)"
fi
# Bundle dir is also intentionally preserved — it's the toolchain payload.
if [[ -d "$BUNDLE" ]]; then
    ok "$BUNDLE preserved (intentional — shared toolchain payload)"
fi

# ── summary ───────────────────────────────────────────────────────────────────
echo
if [[ "$FAIL" -eq 0 ]]; then
    printf "\033[32m== Total: %d passed, %d failed ==\033[0m\n" "$PASS" "$FAIL"
    echo
    echo "Bundle was preserved at $BUNDLE (worker uninstall keeps the shared bundle)."
    echo "To remove the bundle entirely:"
    echo "    sudo rm -rf '$BUNDLE'"
    echo "    sudo dscl . -delete /Users/zisk"
    echo "    sudo dscl . -delete /Groups/zisk"
    exit 0
else
    printf "\033[31m== Total: %d passed, %d failed ==\033[0m\n" "$PASS" "$FAIL"
    echo
    echo "Cleanup hint (if test left a partial install):"
    echo "    sudo $WORKER_INSTALL --uninstall -y"
    echo "    sudo launchctl unload '$PLIST' 2>/dev/null"
    echo "    sudo rm -f '$PLIST' '$NEWSYSLOG' '$SVC_BIN'"
    echo "    sudo rm -rf '$BUNDLE' '$WORK_DIR' '$LOG_DIR'"
    echo "    sudo dscl . -delete /Users/zisk-worker"
    echo "    sudo dscl . -delete /Groups/zisk-worker"
    echo "    sudo dscl . -delete /Users/zisk"
    echo "    sudo dscl . -delete /Groups/zisk"
    exit 1
fi
