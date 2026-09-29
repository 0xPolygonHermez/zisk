#!/bin/bash
# check.sh [snap] [GUEST...]: build each guest, run it and compare its output with
# gold/GUEST.out ("snap" rewrites the gold files instead). Guests are the example
# guests of ziskasm/lang/c/example and the edge-case guests in guests/; with no
# names, every guest that has a gold file. Prints steps and variable cost too.
source "$(dirname "$0")/env.sh"
mode=check; [ "$1" = snap ] && { mode=snap; shift; }
[ $# -gt 0 ] || set -- $(cd "$BENCH/gold" && ls *.out | sed 's/\.out$//')
fail=0
for g in "$@"; do
  src=$BENCH/guests/$g.c; [ -f "$src" ] || src=$ZISK/ziskasm/lang/c/example/$g.c
  guest_cc "$WORK/$g.elf" "$src" 2>"$WORK/cc.err" || { echo "$g: COMPILE ERROR"; head "$WORK/cc.err"; fail=1; continue; }
  rm -f "$WORK/$g.out"
  read st co <<<"$(emu_run "$WORK/$g.elf" "$WORK/$g.out")"
  if [ $mode = snap ]; then cp "$WORK/$g.out" "$BENCH/gold/$g.out"; r=snap
  elif cmp -s "$WORK/$g.out" "$BENCH/gold/$g.out"; then r=ok
  else r=DIFF; fail=1; fi
  printf "%-24s %12s %16s  %s\n" $g "$st" "$co" $r
done
exit $fail
