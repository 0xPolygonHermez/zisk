/*
 * zkvm_stubs.c — C stubs for the Ethereum Foundation zkVM accelerator ABI
 * (zkvm_accelerators.h). Each `zkvm_*` function is a raw-ABI stub with a stable,
 * un-mangled symbol and a placeholder body; during transpilation elf2rom
 * redirects its entry DIRECTLY to the matching native `ziskasm_zkvm_*` routine in
 * ziskasm/zisklib/zkvm/*.zisk. There is NO wrapper and NO intermediate ziskos_*
 * layer: a guest that follows the EF standard calls `zkvm_xxx()` and, in one
 * redirected call, runs the hand-written ziskasm crypto.
 *
 * A guest links EITHER these stubs (the ZisK-accelerated path) OR the portable
 * `zkvm-interface` Rust/C implementation of the same standard symbols — never
 * both; the two are mutually-exclusive definitions of the EF ABI.
 *
 * Stub rules that keep the redirect working (mirrors src/zisklib_stubs.c):
 *   - noinline + used, external linkage: a stable symbol with a real size.
 *   - every argument is touched, so a0..a7 are materialised at the call site for
 *     the redirected routine to read.
 *   - do NOT --strip the guest ELF: elf2rom resolves the stubs by name.
 */
#include "zkvm_accelerators.h"

#define ZKVM_STUB __attribute__((noinline, used))
#define TOUCH(x)  __asm__ volatile("" : : "r"(x) : "memory")

/* ---- hashes (byte-in / byte-out; no marshalling) ----------------------- */
ZKVM_STUB zkvm_status zkvm_keccak256(const uint8_t *data, size_t len,
                                     zkvm_keccak256_hash *output) {
    TOUCH(data); TOUCH(len); TOUCH(output);
    for (int i = 0; i < 32; ++i) output->data[i] = 0xBA;   /* sentinel; redirected away */
    return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_sha256(const uint8_t *data, size_t len,
                                  zkvm_sha256_hash *output) {
    TOUCH(data); TOUCH(len); TOUCH(output);
    for (int i = 0; i < 32; ++i) output->data[i] = 0xB5;
    return ZKVM_EFAIL;
}

/* ---- secp256k1 --------------------------------------------------------- */
ZKVM_STUB zkvm_status zkvm_secp256k1_verify(const zkvm_secp256k1_hash *msg,
                                            const zkvm_secp256k1_signature *sig,
                                            const zkvm_secp256k1_pubkey *pubkey,
                                            bool *verified) {
    TOUCH(msg); TOUCH(sig); TOUCH(pubkey); TOUCH(verified);
    *verified = false;
    return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_secp256k1_ecrecover(const zkvm_secp256k1_hash *msg,
                                               const zkvm_secp256k1_signature *sig,
                                               uint8_t recid,
                                               zkvm_secp256k1_pubkey *output) {
    TOUCH(msg); TOUCH(sig); TOUCH(recid); TOUCH(output);
    for (int i = 0; i < 64; ++i) output->data[i] = 0xBA;
    return ZKVM_EFAIL;
}

/* ---- secp256r1 --------------------------------------------------------- */
ZKVM_STUB zkvm_status zkvm_secp256r1_verify(const zkvm_secp256r1_hash *msg,
                                            const zkvm_secp256r1_signature *sig,
                                            const zkvm_secp256r1_pubkey *pubkey,
                                            bool *verified) {
    TOUCH(msg); TOUCH(sig); TOUCH(pubkey); TOUCH(verified);
    *verified = false;
    return ZKVM_EFAIL;
}

/* ---- blake2f ----------------------------------------------------------- */
ZKVM_STUB zkvm_status zkvm_blake2f(uint32_t rounds, zkvm_blake2f_state *h,
                                   const zkvm_blake2f_message *m,
                                   const zkvm_blake2f_offset *t, uint8_t f) {
    TOUCH(rounds); TOUCH(h); TOUCH(m); TOUCH(t); TOUCH(f);
    return ZKVM_EFAIL;
}

/* ---- modexp (EIP-198; arbitrary-length big-endian byte operands) -------- */
ZKVM_STUB zkvm_status zkvm_modexp(const uint8_t *base, size_t base_len,
                                  const uint8_t *exp, size_t exp_len,
                                  const uint8_t *modulus, size_t mod_len,
                                  uint8_t *output) {
    TOUCH(base); TOUCH(base_len); TOUCH(exp); TOUCH(exp_len);
    TOUCH(modulus); TOUCH(mod_len); TOUCH(output);
    return ZKVM_EFAIL;
}

/* ---- BN254 (alt_bn128) ------------------------------------------------- */
ZKVM_STUB zkvm_status zkvm_bn254_g1_add(const zkvm_bn254_g1_point *p1,
                                        const zkvm_bn254_g1_point *p2,
                                        zkvm_bn254_g1_point *result) {
    TOUCH(p1); TOUCH(p2); TOUCH(result);
    return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_bn254_g1_mul(const zkvm_bn254_g1_point *point,
                                        const zkvm_bn254_scalar *scalar,
                                        zkvm_bn254_g1_point *result) {
    TOUCH(point); TOUCH(scalar); TOUCH(result);
    return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_bn254_pairing(const zkvm_bn254_pairing_pair *pairs,
                                         size_t num_pairs, bool *verified) {
    TOUCH(pairs); TOUCH(num_pairs); TOUCH(verified);
    *verified = false;
    return ZKVM_EFAIL;
}

/* ---- BLS12-381 (EIP-2537) + KZG (EIP-4844) ----------------------------- */
ZKVM_STUB zkvm_status zkvm_bls12_g1_add(const zkvm_bls12_381_g1_point *p1,
                                        const zkvm_bls12_381_g1_point *p2,
                                        zkvm_bls12_381_g1_point *result) {
    TOUCH(p1); TOUCH(p2); TOUCH(result); return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_bls12_g1_msm(const zkvm_bls12_381_g1_msm_pair *pairs,
                                        size_t num_pairs, zkvm_bls12_381_g1_point *result) {
    TOUCH(pairs); TOUCH(num_pairs); TOUCH(result); return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_bls12_g2_add(const zkvm_bls12_381_g2_point *p1,
                                        const zkvm_bls12_381_g2_point *p2,
                                        zkvm_bls12_381_g2_point *result) {
    TOUCH(p1); TOUCH(p2); TOUCH(result); return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_bls12_g2_msm(const zkvm_bls12_381_g2_msm_pair *pairs,
                                        size_t num_pairs, zkvm_bls12_381_g2_point *result) {
    TOUCH(pairs); TOUCH(num_pairs); TOUCH(result); return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_bls12_pairing(const zkvm_bls12_381_pairing_pair *pairs,
                                         size_t num_pairs, bool *verified) {
    TOUCH(pairs); TOUCH(num_pairs); TOUCH(verified); *verified = false; return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_bls12_map_fp_to_g1(const zkvm_bls12_381_fp *field_element,
                                              zkvm_bls12_381_g1_point *result) {
    TOUCH(field_element); TOUCH(result); return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_bls12_map_fp2_to_g2(const zkvm_bls12_381_fp2 *field_element,
                                               zkvm_bls12_381_g2_point *result) {
    TOUCH(field_element); TOUCH(result); return ZKVM_EFAIL;
}
ZKVM_STUB zkvm_status zkvm_kzg_point_eval(const zkvm_kzg_commitment *commitment,
                                          const zkvm_kzg_field_element *z,
                                          const zkvm_kzg_field_element *y,
                                          const zkvm_kzg_proof *proof, bool *verified) {
    TOUCH(commitment); TOUCH(z); TOUCH(y); TOUCH(proof); TOUCH(verified);
    *verified = false; return ZKVM_EFAIL;
}

/* ---- RIPEMD-160 (byte-in; 20-byte digest right-aligned in 32-byte output) --- */
ZKVM_STUB zkvm_status zkvm_ripemd160(const uint8_t *data, size_t len,
                                     zkvm_ripemd160_hash *output) {
    TOUCH(data); TOUCH(len); TOUCH(output);
    for (int i = 0; i < 32; ++i) output->data[i] = 0xB6;
    return ZKVM_EFAIL;
}
