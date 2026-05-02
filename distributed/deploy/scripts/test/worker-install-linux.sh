#!/usr/bin/env bash
# worker-install-linux.sh — end-to-end smoke for the worker install on a Linux host.
#
# Usage (run on Linux, as root):
#   cd <zisk-clone>
#   sudo bash distributed/deploy/scripts/test/worker-install-linux.sh           # safe default
#   sudo bash distributed/deploy/scripts/test/worker-install-linux.sh --start   # also systemctl start
#
# What it does:
#   1. Pre-flight: Linux, root, internet, tarball exists for host arch.
#   2. Runs distributed/deploy/scripts/worker/install.sh --no-mpi --no-enable -y
#      (exercises real ziskup --system download, useradd/groupadd, systemd unit).
#   3. Asserts filesystem layout, ownership, perms, systemd unit content,
#      user/group creation, supplementary group membership.
#   4. With --start: also systemctl start and verify the unit is active with
#      the expected Environment vars. (Without --start, unit is written but
#      not started — safe default.)
#   5. Sweeps state via worker/install.sh --uninstall -y and verifies cleanup.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

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

WORKER_INSTALL="$(cd "${SCRIPT_DIR}/../worker" && pwd)/install.sh"
[[ -x "$WORKER_INSTALL" ]] || { echo "worker/install.sh not found at $WORKER_INSTALL"; exit 1; }

BUNDLE='/opt/zisk'
UNIT='/etc/systemd/system/zisk-worker.service'
WORK_DIR='/var/lib/zisk-worker'
LOG_DIR='/var/log/zisk-worker'
SVC_BIN='/usr/local/bin/zisk-worker'
CONFIG='/etc/zisk/worker.toml'

# ── pre-flight ────────────────────────────────────────────────────────────────
info "Pre-flight"
[[ "$(uname -s)" == "Linux" ]] && ok "host is Linux" || { fail "this test is for Linux only"; exit 1; }
[[ "$EUID" -eq 0 ]] && ok "running as root" || { fail "must run as sudo / root"; exit 1; }

check_github_reachable
check_tarball_exists linux
ARCH="$(resolve_arch)"

# Bail if a previous install is loaded — refuse to clobber.
if systemctl is-enabled zisk-worker >/dev/null 2>&1 || systemctl is-active zisk-worker >/dev/null 2>&1; then
    fail "zisk-worker unit is already enabled/active — refusing to clobber. Run 'sudo $WORKER_INSTALL --uninstall' first."
    exit 1
fi
if [[ -f "$UNIT" ]]; then
    fail "$UNIT already exists — run 'sudo $WORKER_INSTALL --uninstall' first."
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
# Linux x86_64 ships emulator-asm + lib-c sources for runtime ASM build.
if [[ "$ARCH" == "amd64" ]]; then
    [[ -d "$BUNDLE/zisk/emulator-asm" ]] && ok "bundle has zisk/emulator-asm" \
        || fail "bundle missing zisk/emulator-asm"
    [[ -d "$BUNDLE/zisk/lib-c" ]] && ok "bundle has zisk/lib-c" \
        || fail "bundle missing zisk/lib-c"
fi

# ── 3. ownership + perms ──────────────────────────────────────────────────────
info "Bundle ownership + perms"
got=$(stat -c '%U:%G' "$BUNDLE/bin/cargo-zisk")
[[ "$got" == "zisk:zisk" ]] && ok "owner zisk:zisk on cargo-zisk" || fail "bundle owner wrong: got $got"
got=$(stat -c '%a' "$BUNDLE")
[[ "$got" == "750" ]] && ok "$BUNDLE is 0750" || fail "$BUNDLE perm $got, expected 750"
got=$(stat -c '%a' "$BUNDLE/bin/cargo-zisk")
[[ "$got" == "750" ]] && ok "cargo-zisk is 0750" || fail "cargo-zisk perm $got"

# ── 4. system users + groups ──────────────────────────────────────────────────
info "system users + groups"
getent group zisk             >/dev/null 2>&1 && ok "group 'zisk' exists" \
    || fail "group 'zisk' missing"
id zisk                       >/dev/null 2>&1 && ok "user 'zisk' exists" \
    || fail "user 'zisk' missing"
getent group zisk-worker      >/dev/null 2>&1 && ok "group 'zisk-worker' exists" \
    || fail "group 'zisk-worker' missing"
id zisk-worker                >/dev/null 2>&1 && ok "user 'zisk-worker' exists" \
    || fail "user 'zisk-worker' missing"
if id -nG zisk-worker | tr ' ' '\n' | grep -qx zisk; then
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
got=$(stat -c '%U:%G' "$WORK_DIR/cache")
[[ "$got" == "zisk-worker:zisk-worker" ]] && ok "state owner zisk-worker:zisk-worker" \
    || fail "state owner wrong: got $got"

# ── 6. service binary ─────────────────────────────────────────────────────────
info "Service binary"
[[ -f "$SVC_BIN" ]] && ok "$SVC_BIN exists" || fail "$SVC_BIN missing"
[[ -x "$SVC_BIN" ]] && ok "$SVC_BIN is executable" || fail "$SVC_BIN not executable"

# ── 7. systemd unit ───────────────────────────────────────────────────────────
info "systemd unit at $UNIT"
[[ -f "$UNIT" ]] && ok "unit exists" || { fail "unit missing"; exit 1; }
systemctl cat zisk-worker.service >/dev/null 2>&1 \
    && ok "systemctl recognizes the unit" \
    || fail "systemctl can't read the unit"

# Env-var assertions on the unit.
grep -q '^Environment=ZISK_HOME=/opt/zisk$' "$UNIT" \
    && ok "unit sets ZISK_HOME=/opt/zisk" \
    || fail "unit missing ZISK_HOME"
grep -q '^Environment=ZISK_CACHE_DIR=/var/lib/zisk-worker/cache$' "$UNIT" \
    && ok "unit sets ZISK_CACHE_DIR=/var/lib/zisk-worker/cache" \
    || fail "unit missing ZISK_CACHE_DIR"
if grep -qE '^Environment=HOME=' "$UNIT"; then
    fail "unit still sets HOME (should be removed)"
else
    ok "unit does NOT set HOME"
fi
grep -q '^SupplementaryGroups=zisk$' "$UNIT" \
    && ok "unit has SupplementaryGroups=zisk" \
    || fail "unit missing SupplementaryGroups=zisk"
grep -qE '^ReadOnlyPaths=.*/opt/zisk' "$UNIT" \
    && ok "ReadOnlyPaths includes /opt/zisk" \
    || fail "ReadOnlyPaths missing /opt/zisk"
grep -E '^ExecStart=' "$UNIT" | grep -q -- '--proving-key /opt/zisk/provingKey' \
    && ok "ExecStart --proving-key → /opt/zisk/provingKey" \
    || fail "ExecStart --proving-key not in bundle"
grep -q '^# zisk-worker:CONFIG_FILE=/etc/zisk/worker.toml$' "$UNIT" \
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

# ── 8. config ─────────────────────────────────────────────────────────────────
info "Config"
[[ -f "$CONFIG" ]] && ok "$CONFIG exists" || fail "$CONFIG missing"

# ── 9. optional: actually start the daemon and verify env vars are live ───────
if $START_MODE; then
    info "Starting daemon via systemctl (--start mode)"
    systemctl start zisk-worker.service
    sleep 1
    # Worker will fail-loop without a coordinator; we only verify systemd
    # accepted the unit and the env block is correct.
    env_block=$(systemctl show -p Environment zisk-worker.service)
    echo "$env_block"
    if echo "$env_block" | grep -q 'ZISK_HOME=/opt/zisk'; then
        ok "systemd Environment includes ZISK_HOME=/opt/zisk"
    else
        fail "systemd Environment missing ZISK_HOME"
    fi
    if echo "$env_block" | grep -q 'ZISK_CACHE_DIR=/var/lib/zisk-worker/cache'; then
        ok "systemd Environment includes ZISK_CACHE_DIR"
    else
        fail "systemd Environment missing ZISK_CACHE_DIR"
    fi
    systemctl stop zisk-worker.service 2>/dev/null || true
    ok "stopped after verification"
else
    warn "Skipping systemctl start (default mode). Re-run with --start for full verification."
fi

# ── 10. uninstall sweep ───────────────────────────────────────────────────────
info "Running worker install.sh --uninstall -y"
UNINSTALL_OUT=$("$WORKER_INSTALL" --uninstall -y 2>&1) || warn "uninstall exited non-zero"
echo "$UNINSTALL_OUT" | sed 's/^/    /'
echo "$UNINSTALL_OUT" | grep -q 'sudo ziskup --uninstall --system' \
    && ok "uninstall output points operator at 'ziskup --uninstall --system'" \
    || fail "uninstall output missing bundle-removal hint"

[[ ! -f "$UNIT"     ]] && ok "unit removed"            || fail "unit still present"
[[ ! -f "$SVC_BIN"  ]] && ok "service binary removed"  || fail "$SVC_BIN still present"
[[ ! -d "$WORK_DIR" ]] && ok "$WORK_DIR removed"       || warn "$WORK_DIR still present"
if id zisk-worker >/dev/null 2>&1; then
    warn "zisk-worker user still exists after uninstall"
else
    ok "zisk-worker user removed"
fi
# Shared 'zisk' user/group + bundle preserved intentionally.
getent group zisk >/dev/null && ok "shared 'zisk' group preserved (other services may use it)"
[[ -d "$BUNDLE" ]] && ok "$BUNDLE preserved (intentional — shared toolchain payload)"

# ── summary ───────────────────────────────────────────────────────────────────
echo
if [[ "$FAIL" -eq 0 ]]; then
    printf "\033[32m== Total: %d passed, %d failed ==\033[0m\n" "$PASS" "$FAIL"
    exit 0
else
    printf "\033[31m== Total: %d passed, %d failed ==\033[0m\n" "$PASS" "$FAIL"
    echo
    echo "Cleanup hint:"
    echo "    sudo $WORKER_INSTALL --uninstall -y"
    echo "    sudo systemctl stop    zisk-worker 2>/dev/null"
    echo "    sudo systemctl disable zisk-worker 2>/dev/null"
    echo "    sudo rm -f $UNIT $SVC_BIN"
    echo "    sudo rm -rf $BUNDLE $WORK_DIR $LOG_DIR /etc/zisk"
    echo "    sudo userdel  zisk-worker 2>/dev/null"
    echo "    sudo groupdel zisk-worker 2>/dev/null"
    echo "    sudo userdel  zisk        2>/dev/null"
    echo "    sudo groupdel zisk        2>/dev/null"
    exit 1
fi
