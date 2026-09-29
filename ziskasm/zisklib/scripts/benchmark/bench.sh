#!/bin/bash
# bench.sh NAME INPUTS.c 'CALL' [NAME INPUTS.c 'CALL' ...]
# Per-call steps and variable cost of each CALL: driver.c built with N=1 and N=3,
# (run(3) - run(1)) / 2. Extra driver flags go in $DRIVER_FLAGS.
source "$(dirname "$0")/env.sh"
while [ $# -ge 3 ]; do
  for n in 1 3; do
    guest_cc "$WORK/bench$n.elf" "$BENCH/driver.c" -DGUEST="\"$(realpath "$2")\"" -DCALL="$3" -DN=$n $DRIVER_FLAGS || exit 1
  done
  read s1 v1 <<<"$(emu_run "$WORK/bench1.elf" "$WORK/bench.out")"
  read s3 v3 <<<"$(emu_run "$WORK/bench3.elf" "$WORK/bench.out")"
  printf "%-14s steps/call %9d  varcost/call %11d\n" $1 $(( (s3 - s1) / 2 )) $(( (v3 - v1) / 2 ))
  shift 3
done
