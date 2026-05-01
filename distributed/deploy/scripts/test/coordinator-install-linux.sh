#!/usr/bin/env bash
# coordinator-install-linux.sh — end-to-end smoke for the coordinator install
# on a Linux host. Mirror of worker-install-linux.sh with coordinator-specific paths.
#
# Usage (run on Linux, as root):
#   cd <zisk-clone>
#   sudo bash distributed/deploy/scripts/test/coordinator-install-linux.sh           # safe default
#   sudo bash distributed/deploy/scripts/test/coordinator-install-linux.sh --start   # also systemctl start
#
# Differences from worker:
#   - Coordinator unit sets NO ZISK_HOME / ZISK_CACHE_DIR (Stage 1 dropped the
#     HOME= workaround; coordinator code doesn't read $HOME-based paths).
#   - No cache/ or inputs/ subdirs under WORK_DIR.

set -euo pipefail

PASS=0; FAIL=0
ok()   { printf "  \033[32m✓\033[0m %s\n" "$*"; PASS=$((PASS+1)); }
fail() { printf "  \033[31m✗\033[0m %s\n" "$*" >&2; FAIL=$((FAIL+1)); }
info() { printf "\033[1;36m== %s ==\033[0m\n" "$*"; }
warn() { printf "\033[1;33m! %s\033[0m\n" "$*" >&2; }

START_MODE=false
for arg in "$@"; do
    case "$arg" in
        --start) START_MODE=true ;;
        -h|--help)
            awk 'NR==1 {next} /^#/ {print; next} {exit}' "$0"
            exit 0
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COORD_INSTALL="$(cd "${SCRIPT_DIR}/../coordinator" && pwd)/install.sh"
[[ -x "$COORD_INSTALL" ]] || { echo "coordinator/install.sh not found at $COORD_INSTALL"; exit 1; }

BUNDLE='/opt/zisk'
UNIT='/etc/systemd/system/zisk-coordinator.service'
WORK_DIR='/var/lib/zisk-coordinator'
LOG_DIR='/var/log/zisk-coordinator'
SVC_BIN='/usr/local/bin/zisk-coordinator'
CONFIG='/etc/zisk/coordinator.toml'

# ── pre-flight ────────────────────────────────────────────────────────────────
info "Pre-flight"
[[ "$(uname -s)" == "Linux" ]] && ok "host is Linux" || { fail "Linux only"; exit 1; }
[[ "$EUID" -eq 0 ]] && ok "running as root" || { fail "must run as root"; exit 1; }

if curl -fsI -o /dev/null --max-time 5 https://github.com/0xPolygonHermez/zisk/releases/latest 2>/dev/null; then
    ok "github.com reachable"
else
    fail "github.com unreachable"
    exit 1
fi

case "$(uname -m)" in
    aarch64) ARCH=arm64 ;;
    x86_64)  ARCH=amd64 ;;
    *)       ARCH=amd64 ;;
esac
TARBALL_URL="https://github.com/0xPolygonHermez/zisk/releases/latest/download/cargo_zisk_linux_${ARCH}.tar.gz"
if curl -fsIL -o /dev/null --max-time 10 "$TARBALL_URL" 2>/dev/null; then
    ok "release tarball exists for linux_${ARCH}"
else
    fail "release tarball NOT found at $TARBALL_URL"
    exit 1
fi

if systemctl is-enabled zisk-coordinator >/dev/null 2>&1 || systemctl is-active zisk-coordinator >/dev/null 2>&1; then
    fail "zisk-coordinator unit is already enabled/active — refusing to clobber."
    exit 1
fi
if [[ -f "$UNIT" ]]; then
    fail "$UNIT already exists — run 'sudo $COORD_INSTALL --uninstall' first."
    exit 1
fi

# ── 1. install ────────────────────────────────────────────────────────────────
info "Running coordinator install.sh (--no-enable -y)"
"$COORD_INSTALL" --no-enable -y 2>&1 | sed 's/^/    /' || \
    { fail "coordinator install.sh exited non-zero"; exit 1; }

# ── 2. bundle (shared with worker) ────────────────────────────────────────────
info "Bundle under $BUNDLE"
[[ -f "$BUNDLE/bin/zisk-coordinator" ]] && ok "bundle has zisk-coordinator" \
    || fail "bundle missing zisk-coordinator"
[[ -f "$BUNDLE/bin/ziskup" ]] && ok "bundle has ziskup" || fail "bundle missing ziskup"

# ── 3. ownership + perms ──────────────────────────────────────────────────────
got=$(stat -c '%U:%G' "$BUNDLE/bin/zisk-coordinator")
[[ "$got" == "zisk:zisk" ]] && ok "owner zisk:zisk on zisk-coordinator" || fail "owner: $got"

# ── 4. system users + groups ──────────────────────────────────────────────────
info "system users + groups"
getent group zisk             >/dev/null 2>&1 && ok "group 'zisk' exists" \
    || fail "group 'zisk' missing"
id zisk                       >/dev/null 2>&1 && ok "user 'zisk' exists" \
    || fail "user 'zisk' missing"
getent group zisk-coordinator >/dev/null 2>&1 && ok "group 'zisk-coordinator' exists" \
    || fail "group 'zisk-coordinator' missing"
id zisk-coordinator           >/dev/null 2>&1 && ok "user 'zisk-coordinator' exists" \
    || fail "user 'zisk-coordinator' missing"
if id -nG zisk-coordinator | tr ' ' '\n' | grep -qx zisk; then
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
[[ ! -d "$WORK_DIR/.zisk" ]] && ok "$WORK_DIR/.zisk absent (no legacy wrapper)" \
    || fail "legacy $WORK_DIR/.zisk dir present"

# ── 6. service binary ─────────────────────────────────────────────────────────
[[ -f "$SVC_BIN" && -x "$SVC_BIN" ]] && ok "$SVC_BIN present + executable" \
    || fail "$SVC_BIN missing or not executable"

# ── 7. systemd unit ───────────────────────────────────────────────────────────
info "systemd unit at $UNIT"
[[ -f "$UNIT" ]] && ok "unit exists" || { fail "unit missing"; exit 1; }

# Coordinator unit must NOT set any env vars (Stage 1 cleanup).
if grep -qE '^Environment=' "$UNIT"; then
    fail "unit still has Environment= lines (Stage 1 should have removed them)"
else
    ok "unit has NO Environment= lines (correct)"
fi
grep -qE '^WorkingDirectory=/var/lib/zisk-coordinator$' "$UNIT" \
    && ok "WorkingDirectory = /var/lib/zisk-coordinator" \
    || fail "WorkingDirectory wrong"
grep -E '^ExecStart=' "$UNIT" | grep -q -- '--api-port 7000' \
    && ok "ExecStart --api-port = 7000 (default)" \
    || fail "ExecStart --api-port not 7000"

# ── 8. config ─────────────────────────────────────────────────────────────────
[[ -f "$CONFIG" ]] && ok "$CONFIG exists" || fail "$CONFIG missing"

# ── 9. optional: start ────────────────────────────────────────────────────────
if $START_MODE; then
    info "Starting daemon via systemctl (--start mode)"
    systemctl start zisk-coordinator.service
    sleep 1
    if systemctl is-active zisk-coordinator >/dev/null 2>&1; then
        ok "systemctl reports unit active"
    else
        warn "unit not active (may fail-loop without a real workload)"
    fi
    systemctl stop zisk-coordinator.service 2>/dev/null || true
    ok "stopped after verification"
else
    warn "Skipping systemctl start (default mode). Re-run with --start for full verification."
fi

# ── 10. uninstall sweep ───────────────────────────────────────────────────────
info "Running coordinator install.sh --uninstall -y"
"$COORD_INSTALL" --uninstall -y 2>&1 | sed 's/^/    /' || warn "uninstall exited non-zero"

[[ ! -f "$UNIT"     ]] && ok "unit removed"           || fail "unit still present"
[[ ! -f "$SVC_BIN"  ]] && ok "service binary removed" || fail "$SVC_BIN still present"
[[ ! -d "$WORK_DIR" ]] && ok "$WORK_DIR removed"      || warn "$WORK_DIR still present"
if id zisk-coordinator >/dev/null 2>&1; then
    warn "zisk-coordinator user still exists"
else
    ok "zisk-coordinator user removed"
fi
getent group zisk >/dev/null && ok "shared 'zisk' group preserved"
[[ -d "$BUNDLE" ]] && ok "$BUNDLE preserved (intentional)"

# ── summary ───────────────────────────────────────────────────────────────────
echo
if [[ "$FAIL" -eq 0 ]]; then
    printf "\033[32m== Total: %d passed, %d failed ==\033[0m\n" "$PASS" "$FAIL"
    exit 0
else
    printf "\033[31m== Total: %d passed, %d failed ==\033[0m\n" "$PASS" "$FAIL"
    echo
    echo "Cleanup hint:"
    echo "    sudo $COORD_INSTALL --uninstall -y"
    echo "    sudo systemctl stop    zisk-coordinator 2>/dev/null"
    echo "    sudo systemctl disable zisk-coordinator 2>/dev/null"
    echo "    sudo rm -f $UNIT $SVC_BIN"
    echo "    sudo rm -rf $BUNDLE $WORK_DIR $LOG_DIR /etc/zisk"
    echo "    sudo userdel  zisk-coordinator 2>/dev/null"
    echo "    sudo groupdel zisk-coordinator 2>/dev/null"
    echo "    sudo userdel  zisk             2>/dev/null"
    echo "    sudo groupdel zisk             2>/dev/null"
    exit 1
fi
