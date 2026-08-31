/*
 * zisklib.h — C bindings for the ZisK assembly library (ziskasm/zisklib/).
 *
 * A guest program (e.g. a RISC-V ELF built with riscv-none-elf-gcc/g++) includes
 * this header and calls its `ziskos_*` functions. Each function is a raw C-ABI
 * *stub* with a stable, un-mangled symbol name and a placeholder body defined in
 * src/zisklib_stubs.c. During transpilation (`elf2rom`), the stub's entry is
 * redirected to the matching hand-written `zisklib_*` routine in
 * ziskasm/zisklib/<name>.zisk, so the ziskasm implementation runs in the guest's
 * place. The redirect is purely by symbol name (see the REDIRECTS table in
 * transpilers/common/src/elf2rom.rs), so it is language-agnostic: a C or C++
 * caller of `ziskos_keccak` is redirected exactly like the Rust binding.
 *
 * This is the C language binding; the sibling ziskasm/lang/rust/ provides the
 * equivalent Rust binding (the flat ABI and symbol names are identical).
 *
 * ABI conventions
 * ---------------
 *  - 256-bit integers are little-endian `uint64_t[4]` (limb 0 = least
 *    significant). Larger field elements / EC points are contiguous limb arrays
 *    (see each function). Byte-oriented inputs are raw `uint8_t*` + length.
 *  - The first 8 integer/pointer arguments travel in a0..a7 per the RISC-V LP64
 *    calling convention, which is exactly what the .zisk routines read — so a
 *    normal C call site sets up the registers the redirected routine expects.
 *  - Status codes: `0` = success/accept unless noted; nonzero = reject or a
 *    domain-specific error (documented per function).
 *
 * Integration
 * -----------
 *  - Compile src/zisklib_stubs.c into the guest and link it in. No linker-script
 *    change is needed: the .zisk implementation is injected into the ROM by
 *    elf2rom, not linked into the ELF.
 *  - Do NOT --strip-all the guest ELF: elf2rom resolves the stubs by name in the
 *    symbol table, so `.symtab` must survive to the transpile step.
 */

#ifndef ZISKASM_ZISKLIB_H
#define ZISKASM_ZISKLIB_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- demo -------------------------------------------------------------- */

/* a + b, via zisklib_add. A plain sum proves the ziskasm routine ran (the stub
 * returns an argument-dependent sentinel). */
uint64_t ziskos_add(uint64_t a, uint64_t b);

/* ---- hashing ----------------------------------------------------------- */

/* keccak256(input[0..len]) -> output[0..32]. Any len, any alignment. */
void ziskos_keccak(const uint8_t *input, size_t len, uint8_t *output);

/* sha256(input[0..len]) -> output[0..32]. Any len, any alignment. */
void ziskos_sha256(const uint8_t *input, size_t len, uint8_t *output);

/* BLAKE2b compression: permutes `state` (8 u64) in place with the 16-u64
 * `message` block, 2-u64 `offset` counter, and `final_block` flag (0/1). */
void ziskos_blake2b_compress(uint32_t rounds, uint64_t *state,
                             const uint64_t *message, const uint64_t *offset,
                             uint8_t final_block);

/* ---- 256-bit integer arithmetic (little-endian u64[4]) ----------------- */

/* Word-inverse used by the div path. Returns a status; writes result[0..4]. */
uint64_t ziskos_inv256(const uint64_t *a, uint64_t *result);

/* result = a + b; returns the carry-out (0/1). */
uint64_t ziskos_overflowing_add256(const uint64_t *a, const uint64_t *b, uint64_t *result);
/* result = a - b; returns the borrow-out (0/1). */
uint64_t ziskos_overflowing_sub256(const uint64_t *a, const uint64_t *b, uint64_t *result);
/* result = low 256 bits of a * b; returns 1 if the product overflowed 256 bits. */
uint64_t ziskos_overflowing_mul256(const uint64_t *a, const uint64_t *b, uint64_t *result);

/* q = a / b, r = a % b. */
void ziskos_div_rem256(const uint64_t *a, const uint64_t *b, uint64_t *q, uint64_t *r);

/* result = a mod m. */
void ziskos_reduce_mod256(const uint64_t *a, const uint64_t *m, uint64_t *result);
/* result = (a + b) mod m. */
void ziskos_add_mod256(const uint64_t *a, const uint64_t *b, const uint64_t *m, uint64_t *result);
/* result = (a * b) mod m. */
void ziskos_mul_mod256(const uint64_t *a, const uint64_t *b, const uint64_t *m, uint64_t *result);
/* result = a^{-1} mod m; returns a status (nonzero if no inverse exists). */
uint64_t ziskos_inv_mod256(const uint64_t *a, const uint64_t *m, uint64_t *result);
/* result = base^exp mod m. */
void ziskos_pow_mod256(const uint64_t *base, const uint64_t *exp, const uint64_t *m, uint64_t *result);
/* result = low 256 bits of base^exp; returns 1 if it overflowed 256 bits. */
uint64_t ziskos_overflowing_pow256(const uint64_t *base, const uint64_t *exp, uint64_t *result);

/* ---- secp256k1 --------------------------------------------------------- */

/* ECDSA verify. pk = uncompressed point (x||y, u64[8]); z/r/s = u64[4].
 * Returns 1 on a valid signature, 0 otherwise. */
uint64_t ziskos_ecdsa_verify_secp256k1(const uint64_t *pk, const uint64_t *z,
                                       const uint64_t *r, const uint64_t *s);

/* ECDSA public-key recovery. r/s/z = u64[4], recid in {0,1,2,3}. On success
 * writes the recovered point (x||y, u64[8]) to `result` and returns 0; nonzero
 * is an error code. */
uint64_t ziskos_ecdsa_recover_secp256k1(const uint64_t *r, const uint64_t *s,
                                        const uint64_t *z, uint64_t recid,
                                        uint64_t *result);

/* BIP-340 Schnorr verify. pk_x/r/s = u64[4] (already parsed limbs), msg = raw
 * bytes. Returns 1 on a valid signature, 0 otherwise. */
uint64_t ziskos_schnorr_verify_secp256k1(const uint64_t *pk_x, const uint64_t *r,
                                         const uint64_t *s, const uint8_t *msg,
                                         uint64_t msg_len);

/* ---- secp256r1 (P-256) ------------------------------------------------- */

/* ECDSA verify. pk = x||y (u64[8]); z/r/s = u64[4]. Returns 1 if valid. */
uint64_t ziskos_ecdsa_verify_secp256r1(const uint64_t *pk, const uint64_t *z,
                                       const uint64_t *r, const uint64_t *s);

/* ---- BN254 (alt_bn128, EIP-196/197) ------------------------------------ */

/* Pairing check over `n` pairs. g1 = n*(x||y, u64[8]); g2 = n*(x||y in Fp2,
 * u64[16]). Status: 0 accept (product == 1), 1 reject, 2 G1 not canonical,
 * 3 G1 not on curve, 4 G2 not canonical, 5 G2 not on curve, 6 G2 not in subgroup. */
uint64_t ziskos_pairing_check_bn254(const uint64_t *g1, const uint64_t *g2, uint64_t n);

/* ---- BLS12-381 (EIP-2537 / EIP-4844) ----------------------------------- */

/* Pairing check over `n` pairs. g1 = n*(u64[12]); g2 = n*(u64[24]). Status:
 * 0 accept, 1 reject, 2..7 input-validation errors (see the .zisk wrapper). */
uint64_t ziskos_pairing_check_bls12_381(const uint64_t *g1, const uint64_t *g2, uint64_t n);

/* map_to_curve. u in Fp (u64[6]) -> G1 point (u64[12], x||y) in `result`;
 * returns 0 on success, nonzero (1) when u >= p. */
uint64_t ziskos_map_to_curve_g1_bls12_381(const uint64_t *u, uint64_t *result);
/* u in Fp2 (u64[12]) -> G2 point (u64[24]) in `result`; 0 on success. */
uint64_t ziskos_map_to_curve_g2_bls12_381(const uint64_t *u, uint64_t *result);

/* hash_to_curve(msg, dst) -> G2 point (u64[24]) in `result`. */
void ziskos_hash_to_curve_g2_bls12_381(const uint8_t *msg, uint64_t msg_len,
                                       const uint8_t *dst, uint64_t dst_len,
                                       uint64_t *result);

/* BLS signature verify. pk = 48 bytes (compressed G1), sig = 96 bytes
 * (compressed G2), msg = raw bytes. Returns 1 if valid. */
uint64_t ziskos_bls_verify_bls12_381(const uint8_t *pk, const uint8_t *msg,
                                     uint64_t msg_len, const uint8_t *sig);

/* KZG point-evaluation proof verify (EIP-4844). z/y = 32 bytes each,
 * commitment/proof = 48 bytes each (compressed G1). Returns 1 if valid. */
uint64_t ziskos_verify_kzg_proof_bls12_381(const uint8_t *z, const uint8_t *y,
                                           const uint8_t *commitment,
                                           const uint8_t *proof);

/* ---- bigint / MODEXP (EIP-198) ----------------------------------------- */

/* base^exp mod modulus, arbitrary precision. Each operand is a little-endian
 * u64 limb array of the given length; writes result limbs to `result` and
 * returns the number of limbs written (single-U256 moduli and edge cases return
 * 4; larger moduli return ceil(modulus_len/4)*4). */
size_t ziskos_modexp_u64_c(const uint64_t *base, size_t base_len,
                           const uint64_t *exp, size_t exp_len,
                           const uint64_t *modulus, size_t modulus_len,
                           uint64_t *result);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ZISKASM_ZISKLIB_H */
