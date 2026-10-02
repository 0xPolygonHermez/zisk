#!/bin/bash
# check_secp.sh: self-checking zkvm_secp256k1_ecrecover / zkvm_secp256k1_verify
# vectors from secp_glv_vec.py (expected values from Python): the degenerate inputs
# of the 16-entry double-scalar table, zero and equal scalars, results at infinity
# or +-G, equal x coordinates inside the ladder, and random recoveries.
source "$(dirname "$0")/env.sh"
python3 "$BENCH/secp_glv_vec.py" "$WORK/secp_glv.c" 2>/dev/null || exit 1
n=$(grep -c '^  {' "$WORK/secp_glv.c")
guest_cc "$WORK/secp_glv.elf" "$WORK/secp_glv.c" || exit 1
rm -f "$WORK/secp_glv.out"
m=$(emu_run "$WORK/secp_glv.elf" "$WORK/secp_glv.out") || { m="- -"; rm -f "$WORK/secp_glv.out"; }
read st co <<<"$m"
# byte i = 1 if case i failed; byte n = 1 once every case has run
bad=$(od -An -v -tu1 -N$n "$WORK/secp_glv.out" 2>/dev/null | tr -s ' ' '\n' | grep -c '^1$')
done_=$(od -An -tu1 -j$n -N1 "$WORK/secp_glv.out" 2>/dev/null | tr -d ' ')
r=ok; { [ "$bad" = 0 ] && [ "$done_" = 1 ]; } || r="FAIL ($bad of $n cases, done=$done_)"
printf "%-10s %4d cases %12s %16s  %s\n" secp_glv $n "$st" "$co" "$r"
[ "$r" = ok ]
