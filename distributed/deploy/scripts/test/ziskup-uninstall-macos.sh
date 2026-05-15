#!/usr/bin/env bash
# ziskup-uninstall-macos.sh — end-to-end smoke for `ziskup --uninstall --system`
# on macOS. Counterpart of ziskup-uninstall-linux.sh.
#
# Usage (run on macOS, as root):
#   cd <zisk-clone>
#   sudo bash distributed/deploy/scripts/test/ziskup-uninstall-macos.sh
#
# What it does (parallels the Linux test):
#   1. Pre-flight: macOS, root, internet, tarball exists for host arch.
#   2. Snapshots pre-install state (dscl users/groups, bundle dir, etc.).
#   3. Installs both worker and coordinator via their install.sh scripts
#      (with --no-load -y so launchctl doesn't fire).
#   4. Asserts `ziskup --uninstall --system` REFUSES while a com.zisk.*
#      LaunchDaemons plist is still installed.
#   5. Uninstalls both services.
#   6. Runs `ziskup --uninstall --system -y` and asserts manifest entries +
#      'zisk' user/group removed and the bundle dir is gone.
#   7. Snapshots post state and asserts byte-identical match with pre.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./common.sh
source "${SCRIPT_DIR}/common.sh"

WORKER_INSTALL="$(cd "${SCRIPT_DIR}/../worker" && pwd)/install.sh"
COORD_INSTALL="$(cd "${SCRIPT_DIR}/../coordinator" && pwd)/install.sh"
BUNDLE='/Library/Application Support/ZisK'
ZISKUP_BIN="${BUNDLE}/bin/ziskup"
WORKER_PLIST='/Library/LaunchDaemons/com.zisk.worker.plist'
COORD_PLIST='/Library/LaunchDaemons/com.zisk.coordinator.plist'

# ── pre-flight ────────────────────────────────────────────────────────────────
info "Pre-flight"
[[ "$(uname -s)" == "Darwin" ]] && ok "host is macOS"   || { fail "not macOS";    exit 1; }
[[ "$(id -u)"   == "0"      ]] && ok "running as root" || { fail "must be root"; exit 1; }
check_github_reachable
check_tarball_exists darwin

[[ -x "$WORKER_INSTALL" ]] || { fail "worker/install.sh not found at $WORKER_INSTALL"; exit 1; }
[[ -x "$COORD_INSTALL"  ]] || { fail "coordinator/install.sh not found at $COORD_INSTALL"; exit 1; }

for label in com.zisk.worker com.zisk.coordinator; do
    if launchctl print "system/$label" >/dev/null 2>&1; then
        fail "$label is loaded — refusing to clobber. Run uninstalls first."
        exit 1
    fi
done
[[ -f "$WORKER_PLIST" ]] && { fail "$WORKER_PLIST exists";       exit 1; }
[[ -f "$COORD_PLIST"  ]] && { fail "$COORD_PLIST exists";        exit 1; }
[[ -d "$BUNDLE"       ]] && { fail "$BUNDLE exists — uninstall via 'ziskup --uninstall --system'"; exit 1; }

# Snapshot helper (macOS uses dscl for users/groups).
snapshot() {
    {
        dscl . -list /Users         | grep ^zisk    | sort || true
        dscl . -list /Groups        | grep ^zisk    | sort || true
        ls -d "$BUNDLE" /etc/zisk /var/lib/zisk-*    2>&1 | sort || true
        ls /Library/LaunchDaemons/com.zisk.* /usr/local/bin/zisk-* 2>&1 | sort || true
    } > "/tmp/ziskup-uninstall-snap.$1.txt"
}
snapshot pre
ok "pre-install snapshot captured"

# ── 1. install both services ──────────────────────────────────────────────────
# macOS install.sh uses --no-start (no --no-enable on macOS; --no-start writes
# the plist but doesn't launchctl-load it).
info "Installing zisk-worker (--no-mpi --no-start -y)"
"$WORKER_INSTALL" --no-mpi --no-start -y >/dev/null 2>&1 \
    && ok "worker install.sh succeeded" \
    || { fail "worker install.sh failed"; exit 1; }

info "Installing zisk-coordinator (--no-start -y)"
"$COORD_INSTALL" --no-start -y >/dev/null 2>&1 \
    && ok "coordinator install.sh succeeded" \
    || { fail "coordinator install.sh failed"; exit 1; }

[[ -f "$BUNDLE/.zisk-bundle" ]] && ok "ziskup bundle metadata present" \
    || { fail "ziskup bundle metadata missing"; exit 1; }

# ── 2. sibling-safety guard ───────────────────────────────────────────────────
info "Sibling-safety: ziskup --uninstall --system should refuse"
out=$("$ZISKUP_BIN" --uninstall --system 2>&1 || true)
if echo "$out" | grep -q 'Refusing to uninstall'; then
    ok "ziskup --uninstall refused while plists installed"
else
    fail "ziskup --uninstall did NOT refuse"
    echo "$out" | sed 's/^/    /'
fi
echo "$out" | grep -qE '(com\.zisk\.worker|com\.zisk\.coordinator)' \
    && ok "refusal message lists the offending plist(s)" \
    || fail "refusal message did not list plists"

# ── 3. uninstall both services ────────────────────────────────────────────────
info "Uninstalling zisk-worker"
"$WORKER_INSTALL" --uninstall -y >/dev/null 2>&1 \
    && ok "worker --uninstall succeeded" \
    || { fail "worker --uninstall failed"; exit 1; }

info "Uninstalling zisk-coordinator"
"$COORD_INSTALL" --uninstall -y >/dev/null 2>&1 \
    && ok "coordinator --uninstall succeeded" \
    || { fail "coordinator --uninstall failed"; exit 1; }

[[ -d "$BUNDLE" ]] && ok "$BUNDLE preserved after service uninstalls"
dscl . -read /Users/zisk  >/dev/null 2>&1 && ok "'zisk' user preserved" \
    || fail "'zisk' user gone before ziskup --uninstall"
dscl . -read /Groups/zisk >/dev/null 2>&1 && ok "'zisk' group preserved" \
    || fail "'zisk' group gone before ziskup --uninstall"

# ── 4. ziskup --uninstall --system -y ─────────────────────────────────────────
info "Running ziskup --uninstall --system -y"
out=$("$ZISKUP_BIN" --uninstall --system -y 2>&1) || { fail "ziskup --uninstall failed"; echo "$out"; exit 1; }
echo "$out" | sed 's/^/    /'

[[ ! -d "$BUNDLE/bin"  ]] && ok "$BUNDLE/bin removed"  || fail "$BUNDLE/bin still present"
[[ ! -d "$BUNDLE/zisk" ]] && ok "$BUNDLE/zisk removed" || fail "$BUNDLE/zisk still present"
[[ ! -d "$BUNDLE"      ]] && ok "$BUNDLE removed (rmdir succeeded)" \
    || fail "$BUNDLE still present — rmdir didn't succeed"
if dscl . -read /Users/zisk >/dev/null 2>&1; then
    fail "'zisk' user still present after ziskup --uninstall"
else
    ok "'zisk' user removed (.zisk-bundle-driven)"
fi
if dscl . -read /Groups/zisk >/dev/null 2>&1; then
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
    echo "    sudo \"$ZISKUP_BIN\" --uninstall --system -y --force"
    exit 1
fi
