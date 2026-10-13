#!/bin/bash
# Regenerates the optimized bn254 / bls12_381 files of zisklib from their
# pre-optimization sources, which are taken from git (BASE, the last commit
# before this pipeline was applied):
#   prep.py   source edits: batched Miller loop (mlbatch.py), unrolled G1 scalar
#             multiplication (smul.py), unrolled BLS G1 subgroup multiply (sgx.py),
#             pairing / kzg call sites;
#   zopt.py   peephole pass: inlines static-argument Fp / Fp2 leaves, the memcpy /
#             compare helpers and the BE <-> limb conversions;
#   hints.py  unrolled line-coefficient hint reads in miller_loop.zisk;
#   kern.py   kernels.zisk (tc.py tower compiler: Fp6 / Fp12 mul, square, sparse
#             mul, cyclotomic square); strip.py drops the routines it replaces.
# fp2.zisk (both curves) is hand-written and read from the working tree.
#
# usage: build.sh [OUT_DIR]   (default: ziskasm/zisklib, i.e. in place)
set -e
G=$(cd "$(dirname "$0")" && pwd)
R=$(cd "$G/../../../.." && pwd)
BASE=${BASE:-c2324b455}
OUT=${1:-$R/ziskasm/zisklib}
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT

mkdir -p "$T/orig" "$OUT"/{bn254,bls12_381,zkvm}
git -C "$R" archive "$BASE" ziskasm/zisklib/bn254 ziskasm/zisklib/bls12_381 \
    ziskasm/zisklib/zkvm/bn254.zisk ziskasm/zisklib/zkvm/bls12_381.zisk | tar -x -C "$T/orig"
O=$T/orig/ziskasm/zisklib
for c in bn254 bls12_381; do cp "$R/ziskasm/zisklib/$c/fp2.zisk" "$O/$c/fp2.zisk"; done

python3 "$G/prep.py" "$O" "$T/work"
for f in bn254/{fp6,fp12,cyclotomic,final_exp,miller_loop,twist,curve,pairing}.zisk \
         bls12_381/{fp6,fp12,cyclotomic,final_exp,miller_loop,twist,curve,pairing,map,map_g2,hash,kzg,subgroup,verify}.zisk \
         zkvm/bn254.zisk zkvm/bls12_381.zisk; do
  b=$(basename $f .zisk); d=$(dirname $f); P=Z$(echo ${d:0:3}_$b | tr a-z A-Z)
  python3 "$G/zopt.py" "$T/work/$f" "$OUT/$f" $P 2>/dev/null
done
for c in bn254 bls12_381; do
  python3 "$G/hints.py" "$OUT/$c/miller_loop.zisk"
  python3 "$G/kern.py" $c "$OUT/$c/kernels.zisk" 2>/dev/null
  python3 "$G/strip.py" "$OUT/$c/fp6.zisk" zisklib_mul_fp6_$c zisklib_square_fp6_$c zisklib_sparse_mula_fp6_$c zisklib_sparse_mulb_fp6_$c zisklib_sparse_mulc_fp6_$c
  python3 "$G/strip.py" "$OUT/$c/fp12.zisk" zisklib_mul_fp12_$c zisklib_square_fp12_$c zisklib_sparse_mul_fp12_$c
  python3 "$G/strip.py" "$OUT/$c/cyclotomic.zisk" zisklib_square_cyclo_$c
done
echo "built into $OUT"
