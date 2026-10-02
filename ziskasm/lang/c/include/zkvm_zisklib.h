/*
 * zkvm_zisklib.h -- ZisK library functions outside the EF standard: 256-bit
 * integer arithmetic on little-endian u64[4] limbs, secp256k1 / secp256r1
 * signatures, BN254 and BLS12-381 pairings, maps and hashes to curves, BLS and KZG
 * verification, and arbitrary-precision modexp. NOT part of the EF standard.
 *
 * Each function is a zkvmcall thunk (src/zkvm_calls.s): the transpiler turns the
 * call into a jump to the hand-written .zisk routine of the ZisK library
 * (ziskasm/zisklib/), named zisklib_<name>, which follows the RISC-V calling
 * convention. Statuses are each function's own (see below), not zkvm_status.
 *
 * These are raw routines: they do not validate every input. A call that breaks
 * a stated precondition either ends the program at that point (exit 0, so the
 * rest of the guest never runs) or makes the emulator abort; either way no proof
 * of a correct run exists. Check the preconditions before calling.
 */
#ifndef ZKVM_ZISKLIB_H
#define ZKVM_ZISKLIB_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- demo -------------------------------------------------------------- */

/* a + b, via zisklib_add: the smallest library call. */
uint64_t zkvm_zisklib_add(uint64_t a, uint64_t b);

/* ---- 256-bit integer arithmetic (little-endian u64[4]) ----------------- */

/* Word-inverse used by the div path. Returns a status; writes result[0..4]. */
uint64_t zkvm_zisklib_inv256(const uint64_t *a, uint64_t *result);

/* result = a + b; returns the carry-out (0/1). */
uint64_t zkvm_zisklib_overflowing_add256(const uint64_t *a, const uint64_t *b, uint64_t *result);
/* result = a - b; returns the borrow-out (0/1). */
uint64_t zkvm_zisklib_overflowing_sub256(const uint64_t *a, const uint64_t *b, uint64_t *result);
/* result = low 256 bits of a * b; returns 1 if the product overflowed 256 bits. */
uint64_t zkvm_zisklib_overflowing_mul256(const uint64_t *a, const uint64_t *b, uint64_t *result);

/* q = a / b, r = a % b. Requires b != 0: b == 0 ends the program. */
void zkvm_zisklib_div_rem256(const uint64_t *a, const uint64_t *b, uint64_t *q, uint64_t *r);

/* The modular functions below require m != 0: m == 0 aborts the emulator (the
 * arith256_mod precompile divides by m). */

/* result = a mod m. */
void zkvm_zisklib_reduce_mod256(const uint64_t *a, const uint64_t *m, uint64_t *result);
/* result = (a + b) mod m. */
void zkvm_zisklib_add_mod256(const uint64_t *a, const uint64_t *b, const uint64_t *m, uint64_t *result);
/* result = (a * b) mod m. */
void zkvm_zisklib_mul_mod256(const uint64_t *a, const uint64_t *b, const uint64_t *m, uint64_t *result);
/* result = a^{-1} mod m; returns 1 and writes `result` if the inverse exists,
 * 0 otherwise. Requires m != 0. */
uint64_t zkvm_zisklib_inv_mod256(const uint64_t *a, const uint64_t *m, uint64_t *result);
/* result = base^exp mod m. Requires m >= 2 (with m == 1 and exp == 0 it returns
 * 1, not 0). */
void zkvm_zisklib_pow_mod256(const uint64_t *base, const uint64_t *exp, const uint64_t *m, uint64_t *result);
/* result = low 256 bits of base^exp; returns 1 if it overflowed 256 bits. */
uint64_t zkvm_zisklib_overflowing_pow256(const uint64_t *base, const uint64_t *exp, uint64_t *result);

/* ---- secp256k1 --------------------------------------------------------- */

/* ECDSA verify. pk = uncompressed point (x||y, u64[8]); z/r/s = u64[4].
 * Returns 1 on a valid signature, 0 otherwise. */
uint64_t zkvm_zisklib_ecdsa_verify_secp256k1(const uint64_t *pk, const uint64_t *z,
                                             const uint64_t *r, const uint64_t *s);

/* ECDSA public-key recovery. r/s/z = u64[4], recid in {0,1} (the x = r + n branch
 * that ids 2/3 select is not supported; they return error 3, matching the ziskos
 * reference ecdsa_recover_secp256k1).
 * On success writes the recovered point (x||y, u64[8]) to `result` and returns 0; nonzero
 * is an error code. */
uint64_t zkvm_zisklib_ecdsa_recover_secp256k1(const uint64_t *r, const uint64_t *s,
                                              const uint64_t *z, uint64_t recid,
                                              uint64_t *result);

/* BIP-340 Schnorr verify. pk_x/r/s = u64[4] (already parsed limbs), msg = raw
 * bytes. Returns 1 on a valid signature, 0 otherwise. */
uint64_t zkvm_zisklib_schnorr_verify_secp256k1(const uint64_t *pk_x, const uint64_t *r,
                                               const uint64_t *s, const uint8_t *msg,
                                               uint64_t msg_len);

/* ---- secp256r1 (P-256) ------------------------------------------------- */

/* ECDSA verify. pk = x||y (u64[8]); z/r/s = u64[4]. Returns 1 if valid. */
uint64_t zkvm_zisklib_ecdsa_verify_secp256r1(const uint64_t *pk, const uint64_t *z,
                                             const uint64_t *r, const uint64_t *s);

/* ---- BN254 (alt_bn128, EIP-196/197) ------------------------------------ */

/* Pairing check over `n` pairs. g1 = n*(x||y, u64[8]); g2 = n*(x||y in Fp2,
 * u64[16]). Status: 0 accept (product == 1), 1 reject, 2 G1 not canonical,
 * 3 G1 not on curve, 4 G2 not canonical, 5 G2 not on curve, 6 G2 not in subgroup. */
uint64_t zkvm_zisklib_pairing_check_bn254(const uint64_t *g1, const uint64_t *g2, uint64_t n);

/* ---- BLS12-381 (EIP-2537 / EIP-4844) ----------------------------------- */

/* Pairing check over `n` pairs. g1 = n*(u64[12]); g2 = n*(u64[24]). Status:
 * 0 accept, 1 reject, 2..7 input-validation errors (see the .zisk wrapper). */
uint64_t zkvm_zisklib_pairing_check_bls12_381(const uint64_t *g1, const uint64_t *g2, uint64_t n);

/* map_to_curve. u in Fp (u64[6]) -> G1 point (u64[12], x||y) in `result`;
 * returns 0 on success, nonzero (1) when u >= p. */
uint64_t zkvm_zisklib_map_to_curve_g1_bls12_381(const uint64_t *u, uint64_t *result);
/* u in Fp2 (u64[12]) -> G2 point (u64[24]) in `result`; 0 on success. */
uint64_t zkvm_zisklib_map_to_curve_g2_bls12_381(const uint64_t *u, uint64_t *result);

/* hash_to_curve(msg, dst) -> G2 point (u64[24]) in `result`. */
void zkvm_zisklib_hash_to_curve_g2_bls12_381(const uint8_t *msg, uint64_t msg_len,
                                             const uint8_t *dst, uint64_t dst_len,
                                             uint64_t *result);

/* BLS signature verify. pk = 48 bytes (compressed G1), sig = 96 bytes
 * (compressed G2), msg = raw bytes. Returns 1 if valid. */
uint64_t zkvm_zisklib_bls_verify_bls12_381(const uint8_t *pk, const uint8_t *msg,
                                           uint64_t msg_len, const uint8_t *sig);

/* KZG point-evaluation proof verify (EIP-4844). z/y = 32 bytes each,
 * commitment/proof = 48 bytes each (compressed G1). Returns 1 if valid. */
uint64_t zkvm_zisklib_verify_kzg_proof_bls12_381(const uint8_t *z, const uint8_t *y,
                                                 const uint8_t *commitment,
                                                 const uint8_t *proof);

/* ---- bigint / MODEXP (EIP-198) ----------------------------------------- */

/* base^exp mod modulus, arbitrary precision. Each operand is a little-endian
 * u64 limb array of the given length; writes result limbs to `result` and
 * returns the number of limbs written (single-U256 moduli and edge cases return
 * 4; larger moduli return ceil(modulus_len/4)*4).
 * Requires 1 <= base_len, exp_len, modulus_len <= 132 (1056 bytes): a zero-length
 * modulus aborts the emulator, and a zero-length base or an operand over 132
 * limbs ends the program. `result` must hold max(4, ceil(modulus_len/4)*4)
 * limbs. */
size_t zkvm_zisklib_modexp_u64_c(const uint64_t *base, size_t base_len,
                                 const uint64_t *exp, size_t exp_len,
                                 const uint64_t *modulus, size_t modulus_len,
                                 uint64_t *result);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ZKVM_ZISKLIB_H */
