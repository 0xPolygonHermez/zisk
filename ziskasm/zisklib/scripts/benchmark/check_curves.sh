#!/bin/bash
# check_curves.sh [SEED...]: bn254 / BLS12-381 / KZG vectors of curves_vec.py (needs
# py_ecc; default seeds 1 2 3). The guest checks each case against py_ecc and hashes
# every status and output into a transcript. Set REF_ZISKEMU to an emulator built
# from another commit to also require identical transcripts.
source "$(dirname "$0")/env.sh"
[ $# -gt 0 ] || set -- 1 2 3
fail=0
for seed in "$@"; do
  python3 "$BENCH/curves_vec.py" $seed "$WORK/curves_vec$seed.c" >/dev/null || exit 1
  guest_cc "$WORK/curves$seed.elf" "$WORK/curves_vec$seed.c" || exit 1
  for E in "$ZISKEMU" $REF_ZISKEMU; do
    rm -f "$WORK/curves.out"
    m=$(emu_run "$WORK/curves$seed.elf" "$WORK/curves.out" "$E") || { m="- -"; rm -f "$WORK/curves.out"; }
    read st co <<<"$m"
    # [fails u32][first failing case u32][ncases u32][0 u32][keccak(transcript)]
    read f first n <<<"$(od -An -tu4 -N12 "$WORK/curves.out")"
    h=$(od -An -tx1 -j16 -N32 "$WORK/curves.out" | tr -d ' \n')
    [ "$f" = 0 ] || fail=1; [ "$first" = 4294967295 ] && first=none
    [ "$E" = "$ZISKEMU" ] && h0=$h || [ "$h" = "$h0" ] || { fail=1; h="$h DIFFERS"; }
    printf "seed %-3s %-8s cases %3s fails %s (first %s)  steps %s cost %s  transcript %s\n" \
      $seed $([ "$E" = "$ZISKEMU" ] && echo test || echo ref) "$n" "$f" "$first" "$st" "$co" "$h"
  done
done
exit $fail
