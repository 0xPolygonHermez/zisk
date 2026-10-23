#!/usr/bin/env bash
#
# run.sh — build every patch-testing guest against the ziskos in this tree and
# execute it, reporting PASS/FAIL per patch.
#
# Usage: patch-testing/run.sh [--host] [guest-dir ...]
#
#   (default)  build each guest for the ZisK target with cargo-zisk and run the
#              ELF in ziskemu. This exercises the accelerated code paths and is
#              what catches a patch that no longer links against ziskos.
#   --host     build and run each guest natively. The patched crates fall back to
#              their upstream code there, so this validates the test vectors.
#
# With no guest names, every guest directory is run.
#
# Environment:
#   CARGO_ZISK, ZISKEMU   binaries to use (default: built from this tree)
#   MAX_STEPS             ziskemu step limit (default: ziskemu's own)
#   REPORT_MD             also write a Markdown summary table to this path
#   XFAIL                 space-separated guests known to be broken: their
#                         failure is reported as XFAIL and does not fail the run
#                         (an XFAIL guest that passes is reported as XPASS)
#
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/.." && pwd)"
LOG_DIR="$HERE/target/logs"
mkdir -p "$LOG_DIR"

MODE=zisk
if [[ "${1:-}" == "--host" ]]; then
  MODE=host
  shift
fi

if [[ $# -gt 0 ]]; then
  GUESTS=("$@")
else
  GUESTS=()
  for d in "$HERE"/*/; do
    d="$(basename "$d")"
    [[ "$d" == common || "$d" == target ]] && continue
    [[ -f "$HERE/$d/Cargo.toml" ]] && GUESTS+=("$d")
  done
fi

if [[ "$MODE" == zisk ]]; then
  if [[ -z "${CARGO_ZISK:-}" || -z "${ZISKEMU:-}" ]]; then
    echo "::group::Build cargo-zisk and ziskemu from $REPO"
    (cd "$REPO" && cargo build --release -p cargo-zisk -p ziskemu --bin cargo-zisk --bin ziskemu) || exit 1
    echo "::endgroup::"
  fi
  CARGO_ZISK="${CARGO_ZISK:-$REPO/target/release/cargo-zisk}"
  ZISKEMU="${ZISKEMU:-$REPO/target/release/ziskemu}"
fi

# cargo-zisk always builds into ./target/elf, so point every guest's target/ at
# one shared directory: ziskos and the common deps are then compiled once.
share_target_dir() {
  local dir="$1"
  if [[ ! -e "$dir/target" ]]; then
    ln -s ../target "$dir/target"
  fi
}

declare -a RESULTS=()
failed=0

for g in "${GUESTS[@]}"; do
  dir="$HERE/$g"
  pkg="$(sed -n 's/^name = "\(.*\)"/\1/p' "$dir/Cargo.toml" | head -1)"
  build_log="$LOG_DIR/$g.$MODE.build.log"
  run_log="$LOG_DIR/$g.$MODE.run.log"
  share_target_dir "$dir"

  echo "::group::$g ($MODE)"
  status=PASS
  detail=""
  if [[ "$MODE" == zisk ]]; then
    if (cd "$dir" && "$CARGO_ZISK" build --release) >"$build_log" 2>&1; then
      elf="$HERE/target/elf/riscv64ima-zisk-zkvm-elf/release/$pkg"
      emu_args=(-e "$elf")
      [[ -n "${MAX_STEPS:-}" ]] && emu_args+=(-n "$MAX_STEPS")
      if "$ZISKEMU" "${emu_args[@]}" >"$run_log" 2>&1 && grep -q "\] PASS$" "$run_log"; then
        detail="$(grep -oE '[0-9]+ passed' "$run_log" | tail -1)"
      else
        status=FAIL
        detail="run: $(grep -E 'FAIL|panicked|Error' "$run_log" | head -1)"
      fi
    else
      status=FAIL
      # The usual breakage: the patch imports a symbol ziskos no longer exports.
      undefined="$(grep -oE 'undefined symbol: [A-Za-z0-9_]+' "$build_log" | sort -u | tr '\n' ' ')"
      detail="build: ${undefined:-$(grep -m1 -E '^error' "$build_log")}"
    fi
  else
    if (cd "$dir" && cargo run --release -q) >"$run_log" 2>&1 && grep -q "\] PASS$" "$run_log"; then
      detail="$(grep -oE '[0-9]+ passed' "$run_log" | tail -1)"
    else
      status=FAIL
      detail="$(grep -E 'FAIL|panicked|^error' "$run_log" | head -1)"
    fi
  fi
  cat "$run_log" 2>/dev/null | grep -E '^\[|got:|expected:' | head -40
  if [[ " ${XFAIL:-} " == *" $g "* ]]; then
    [[ "$status" == FAIL ]] && status=XFAIL || status=XPASS
  fi
  [[ "$status" == FAIL ]] && failed=$((failed + 1))
  RESULTS+=("$g|$status|$detail")
  echo "::endgroup::"
done

echo
printf '%-28s %-5s %s\n' GUEST RESULT DETAIL
for r in "${RESULTS[@]}"; do
  IFS='|' read -r g s d <<<"$r"
  printf '%-28s %-5s %s\n' "$g" "$s" "$d"
done
echo "Logs: $LOG_DIR"

if [[ -n "${REPORT_MD:-}" ]]; then
  {
    echo "### Patch testing ($MODE, ziskos @ $(git -C "$REPO" rev-parse --short HEAD 2>/dev/null || echo unknown))"
    echo
    echo "| Guest | Result | Detail |"
    echo "|---|---|---|"
    for r in "${RESULTS[@]}"; do
      IFS='|' read -r g s d <<<"$r"
      case "$s" in
        PASS) icon="✅" ;;
        XFAIL) icon="⚠️" ;;
        XPASS) icon="🎉" ;;
        *) icon="❌" ;;
      esac
      echo "| \`$g\` | $icon $s | ${d//|/\\|} |"
    done
  } >"$REPORT_MD"
fi

exit $((failed > 0))
