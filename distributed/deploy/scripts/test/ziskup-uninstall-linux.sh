#!/usr/bin/env bash
# ziskup-uninstall-linux.sh — end-to-end smoke for `ziskup --uninstall --system`.
#
# Usage (run on Linux, as root):
#   cd <zisk-clone>
#   sudo bash distributed/deploy/scripts/test/ziskup-uninstall-linux.sh
#
# What it does:
#   1. Pre-flight: Linux, root, internet, tarball exists for host arch.
#   2. Snapshots pre-install state (zisk users/groups, bundle dir, etc.).
#   3. Installs both worker and coordinator via their install.sh scripts.
#   4. Asserts `ziskup --uninstall --system` REFUSES while a zisk-* service
#      is still installed (sibling-safety guard).
#   5. Uninstalls both services.
#   6. Runs `ziskup --uninstall --system -y` and asserts:
#      - manifest entries removed (bin/, zisk/);
#      - 'zisk' user/group removed (because ziskup created them — .zisk-bundle
#        recorded created_user/created_group);
#      - empty $BUNDLE rmdir'd.
#   7. Snapshots post state and asserts byte-identical match with pre.
#
# This complements worker/coordinator-install-linux.sh which only test the
# per-service install.sh --uninstall path. This test exercises the bundle
# uninstall and the full pre/post restore.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

WORKER_INSTALL="$(cd "${SCRIPT_DIR}/../worker" && pwd)/install.sh"
COORD_INSTALL="$(cd "${SCRIPT_DIR}/../coordinator" && pwd)/install.sh"
ZISKUP_BIN='/opt/zisk/bin/ziskup'
BUNDLE='/opt/zisk'

# ── pre-flight ────────────────────────────────────────────────────────────────
info "Pre-flight"
[[ "$(uname -s)" == "Linux" ]]   && ok "host is Linux"   || { fail "not Linux";   exit 1; }
[[ "$(id -u)"   == "0"      ]]   && ok "running as root" || { fail "must be root"; exit 1; }
check_github_reachable
check_tarball_exists linux

[[ -x "$WORKER_INSTALL" ]] || { fail "worker/install.sh not found at $WORKER_INSTALL"; exit 1; }
[[ -x "$COORD_INSTALL"  ]] || { fail "coordinator/install.sh not found at $COORD_INSTALL"; exit 1; }

# Refuse to clobber a real install.
for svc in zisk-worker zisk-coordinator; do
    if systemctl is-enabled "$svc" 2>/dev/null | grep -q enabled \
       || systemctl is-active "$svc" 2>/dev/null | grep -q active; then
        fail "$svc is already enabled/active — refusing to clobber. Run uninstalls first."
        exit 1
    fi
done
[[ -f /etc/systemd/system/zisk-worker.service      ]] && { fail "zisk-worker.service exists";      exit 1; }
[[ -f /etc/systemd/system/zisk-coordinator.service ]] && { fail "zisk-coordinator.service exists"; exit 1; }
[[ -d "$BUNDLE" ]] && { fail "$BUNDLE exists — uninstall it first via 'ziskup --uninstall --system'"; exit 1; }

# Snapshot helper — captures everything our changes touch.
snapshot() {
    {
        getent passwd | grep '^zisk' | sort || true
        getent group  | grep '^zisk' | sort || true
        ls -d /opt/zisk /etc/zisk /var/lib/zisk-* 2>&1 | sort || true
        ls /etc/systemd/system/zisk-* /usr/local/bin/zisk-* 2>&1 | sort || true
    } > "/tmp/ziskup-uninstall-snap.$1.txt"
}
snapshot pre
ok "pre-install snapshot captured"

# ── 1. install both services ──────────────────────────────────────────────────
info "Installing zisk-worker (--no-mpi --no-enable -y)"
"$WORKER_INSTALL" --no-mpi --no-enable -y >/dev/null 2>&1 \
    && ok "worker install.sh succeeded" \
    || { fail "worker install.sh failed"; exit 1; }

info "Installing zisk-coordinator (--no-enable -y)"
"$COORD_INSTALL" --no-enable -y >/dev/null 2>&1 \
    && ok "coordinator install.sh succeeded" \
    || { fail "coordinator install.sh failed"; exit 1; }

# Sanity: bundle metadata was written.
[[ -f "$BUNDLE/.zisk-bundle" ]] && ok "ziskup bundle metadata present at $BUNDLE/.zisk-bundle" \
    || { fail "ziskup bundle metadata missing — uninstall would fall back to default subdir list"; exit 1; }

# ── 2. ziskup --uninstall must REFUSE while services are installed ────────────
info "Sibling-safety: ziskup --uninstall --system should refuse"
out=$("$ZISKUP_BIN" --uninstall --system 2>&1 || true)
if echo "$out" | grep -q 'Refusing to uninstall'; then
    ok "ziskup --uninstall refused while services installed"
else
    fail "ziskup --uninstall did NOT refuse — sibling-safety guard missing"
    echo "$out" | sed 's/^/    /'
fi
echo "$out" | grep -qE '(zisk-worker|zisk-coordinator)' \
    && ok "refusal message lists the offending service(s)" \
    || fail "refusal message did not list services"

# ── 3. uninstall both services ────────────────────────────────────────────────
info "Uninstalling zisk-worker"
"$WORKER_INSTALL" --uninstall -y >/dev/null 2>&1 \
    && ok "worker --uninstall succeeded" \
    || { fail "worker --uninstall failed"; exit 1; }

info "Uninstalling zisk-coordinator"
"$COORD_INSTALL" --uninstall -y >/dev/null 2>&1 \
    && ok "coordinator --uninstall succeeded" \
    || { fail "coordinator --uninstall failed"; exit 1; }

# Bundle and zisk user are intentionally still around.
[[ -d "$BUNDLE" ]] && ok "$BUNDLE preserved after service uninstalls (intentional)"
getent passwd zisk >/dev/null && ok "'zisk' user preserved after service uninstalls"
getent group  zisk >/dev/null && ok "'zisk' group preserved after service uninstalls"

# ── 4. ziskup --uninstall --system -y (the actual bundle removal) ─────────────
info "Running ziskup --uninstall --system -y"
out=$("$ZISKUP_BIN" --uninstall --system -y 2>&1) || { fail "ziskup --uninstall failed"; echo "$out"; exit 1; }
echo "$out" | sed 's/^/    /'

# Bundle subdirs gone.
[[ ! -d "$BUNDLE/bin"  ]] && ok "$BUNDLE/bin removed"  || fail "$BUNDLE/bin still present"
[[ ! -d "$BUNDLE/zisk" ]] && ok "$BUNDLE/zisk removed" || fail "$BUNDLE/zisk still present"
# Bundle root gone (it was empty after subdirs removed).
[[ ! -d "$BUNDLE" ]] && ok "$BUNDLE removed (rmdir succeeded)" \
    || fail "$BUNDLE still present — rmdir didn't succeed"
# zisk user/group gone (because .zisk-bundle had created_user=zisk, created_group=zisk).
if id zisk >/dev/null 2>&1; then
    fail "'zisk' user still present (.zisk-bundle should have triggered removal)"
else
    ok "'zisk' user removed (.zisk-bundle-driven)"
fi
if getent group zisk >/dev/null 2>&1; then
    fail "'zisk' group still present"
else
    ok "'zisk' group removed (.zisk-bundle-driven)"
fi

# ── 5. full pre/post restore ──────────────────────────────────────────────────
snapshot post
if diff -q /tmp/ziskup-uninstall-snap.pre.txt /tmp/ziskup-uninstall-snap.post.txt >/dev/null 2>&1; then
    ok "pre vs post: byte-identical (full state restore)"
else
    fail "pre vs post differ — system not fully restored:"
    diff -u /tmp/ziskup-uninstall-snap.pre.txt /tmp/ziskup-uninstall-snap.post.txt | sed 's/^/    /'
fi

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
    echo "    sudo $COORD_INSTALL  --uninstall -y"
    echo "    sudo $ZISKUP_BIN --uninstall --system -y --force"
    exit 1
fi
