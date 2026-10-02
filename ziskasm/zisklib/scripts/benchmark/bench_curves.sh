#!/bin/bash
# bench_curves.sh: per-call steps / variable cost of the bn254, BLS12-381 and KZG
# zkvm_* functions. Inputs are the seed-1 vectors of curves_vec.py (needs py_ecc),
# generated into $WORK on first use.
source "$(dirname "$0")/env.sh"
I=$WORK/curves_vec1.c
[ -f "$I" ] || python3 "$BENCH/curves_vec.py" 1 "$I" >/dev/null || exit 1
export DRIVER_FLAGS=-DGUEST_HAS_OB
exec "$BENCH/bench.sh" \
 bn_add    $I 'zkvm_bn254_g1_add((const void*)i0_0,(const void*)i0_1,(void*)ob)' \
 bn_mul    $I 'zkvm_bn254_g1_mul((const void*)i13_0,(const void*)i13_1,(void*)ob)' \
 bn_pair2  $I 'zkvm_bn254_pairing((const void*)i28_0,2,&okb)' \
 bn_pair9  $I 'zkvm_bn254_pairing((const void*)i91_0,9,&okb)' \
 bl_g1add  $I 'zkvm_bls12_g1_add((const void*)i37_0,(const void*)i37_1,(void*)ob)' \
 bl_g2add  $I 'zkvm_bls12_g2_add((const void*)i46_0,(const void*)i46_1,(void*)ob)' \
 bl_g1msm1 $I 'zkvm_bls12_g1_msm((const void*)i53_0,1,(void*)ob)' \
 bl_g1msm3 $I 'zkvm_bls12_g1_msm((const void*)i55_0,3,(void*)ob)' \
 bl_g2msm1 $I 'zkvm_bls12_g2_msm((const void*)i58_0,1,(void*)ob)' \
 bl_pair2  $I 'zkvm_bls12_pairing((const void*)i62_0,2,&okb)' \
 bl_pair9  $I 'zkvm_bls12_pairing((const void*)i95_0,9,&okb)' \
 bl_map1   $I 'zkvm_bls12_map_fp_to_g1((const void*)i68_0,(void*)ob)' \
 bl_map2   $I 'zkvm_bls12_map_fp2_to_g2((const void*)i72_0,(void*)ob)' \
 kzg_bad   $I 'zkvm_kzg_point_eval((const void*)i78_0,(const void*)i78_1,(const void*)i78_2,(const void*)i78_3,&okb)' \
 kzg_ok    $I 'zkvm_kzg_point_eval((const void*)i85_0,(const void*)i85_1,(const void*)i85_2,(const void*)i85_3,&okb)'
