#!/bin/bash
# bench_accel.sh: per-call steps / variable cost of the hash, secp and modexp
# zkvm_* functions on the fixed inputs of accel_inputs.c.
D=$(dirname "$0"); I=$(cd "$D" && pwd)/accel_inputs.c
exec "$D/bench.sh" \
 keccak_0       $I 'zkvm_keccak256(HB,0,(void*)ob)' \
 keccak_32      $I 'zkvm_keccak256(HB,32,(void*)ob)' \
 keccak_64      $I 'zkvm_keccak256(HB,64,(void*)ob)' \
 keccak_136     $I 'zkvm_keccak256(HB,136,(void*)ob)' \
 keccak_300     $I 'zkvm_keccak256(HB,300,(void*)ob)' \
 keccakf1600    $I 'zkvm_keccak_f1600((uint64_t*)HB)' \
 sha256_32      $I 'zkvm_sha256(HB,32,(void*)ob)' \
 sha256_64      $I 'zkvm_sha256(HB,64,(void*)ob)' \
 sha256_300     $I 'zkvm_sha256(HB,300,(void*)ob)' \
 ripemd_32      $I 'zkvm_ripemd160(HB,32,(void*)ob)' \
 ripemd_300     $I 'zkvm_ripemd160(HB,300,(void*)ob)' \
 blake2f_12     $I 'zkvm_blake2f(12,&BS,&BM,&BT,1)' \
 k1_verify      $I 'zkvm_secp256k1_verify(&K1MSG,&K1SIG,&K1PUB,&okb)' \
 k1_ecrecover   $I 'zkvm_secp256k1_ecrecover(&RMSG,&RSIG,1,&PKO)' \
 r1_verify      $I 'zkvm_secp256r1_verify(&R1MSG,&R1SIG,&R1PUB,&okb)' \
 mx_32_32_32    $I 'zkvm_modexp(v2_b,32,v2_e,32,v2_m,32,ob)' \
 mx_37_19_48    $I 'zkvm_modexp(v3_b,37,v3_e,19,v3_m,48,ob)' \
 mx_64_32_64    $I 'zkvm_modexp(B64,64,E32,32,M64,64,ob)' \
 mx_128_128_128 $I 'zkvm_modexp(B128,128,E128,128,M128,128,ob)' \
 mx_rsa_e65537  $I 'zkvm_modexp(B256,256,E3,3,M256,256,ob)' \
 mx_rsa_e256B   $I 'zkvm_modexp(B256,256,E256,256,M256,256,ob)' \
 mx_512_64_512  $I 'zkvm_modexp(B512,512,E64,64,M512,512,ob)'
