#!/bin/bash
# profile.sh INPUTS.c 'CALL' [N] [TOP]: where the steps of N (default 10) calls of
# CALL go, per zisklib routine (ziskemu -H pc histogram folded by profile.py).
source "$(dirname "$0")/env.sh"
guest_cc "$WORK/prof.elf" "$BENCH/driver.c" -DGUEST="\"$(realpath "$1")\"" -DCALL="$2" -DN=${3:-10} $DRIVER_FLAGS || exit 1
(cd "$ZISK" && cargo run -q --release -p zisk-asm --example dump_syms) > "$WORK/syms.txt" || exit 1
"$ZISKEMU" -e "$WORK/prof.elf" -i "$WORK/empty.bin" -X -H 100000 2>&1 | grep -v "MEM MONITOR" > "$WORK/prof_hist.txt"
rc=${PIPESTATUS[0]}
if [ "$rc" -ne 0 ]; then
  echo "profile.sh: ziskemu failed (exit $rc); not folding a partial histogram:" >&2
  tail -n 5 "$WORK/prof_hist.txt" | sed 's/^/  /' >&2
  exit 1
fi
python3 "$BENCH/profile.py" "$WORK/syms.txt" "$WORK/prof_hist.txt" ${4:-25}
