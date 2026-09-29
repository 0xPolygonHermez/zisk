#!/bin/bash
# check_modexp.sh: self-checking zkvm_modexp vectors (expected values from Python
# pow): modexp_vec.py (up to 64-byte moduli, edge cases) and modexp_vec_long.py
# (49..1025-byte moduli, odd and even).
source "$(dirname "$0")/env.sh"
python3 "$BENCH/modexp_vec.py" "$WORK/mx_short.c" >/dev/null
python3 "$BENCH/modexp_vec_long.py" "[49,63,64,65,96,97,128,200,256,257,384,512,779,1000,1024]" "$WORK/mx_long.c" >/dev/null
python3 "$BENCH/modexp_vec_long.py" "[1025]" "$WORK/mx_1025.c" >/dev/null
fail=0
for g in mx_short mx_long mx_1025; do
  n=$(grep -c 'zkvm_modexp(' "$WORK/$g.c")
  guest_cc "$WORK/$g.elf" "$WORK/$g.c" || exit 1
  rm -f "$WORK/$g.out"
  read st co <<<"$(emu_run "$WORK/$g.elf" "$WORK/$g.out")"
  # byte i = 1 if case i failed; byte n = 1 once every case has run
  bad=$(od -An -v -tu1 -N$n "$WORK/$g.out" | tr -s ' ' '\n' | grep -c '^1$')
  done_=$(od -An -tu1 -j$n -N1 "$WORK/$g.out" | tr -d ' ')
  r=ok; { [ "$bad" = 0 ] && [ "$done_" = 1 ]; } || { r="FAIL ($bad of $n cases, done=$done_)"; fail=1; }
  printf "%-10s %4d cases %12s %16s  %s\n" $g $n "$st" "$co" "$r"
done
exit $fail
