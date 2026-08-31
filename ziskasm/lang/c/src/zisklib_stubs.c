/*
 * zisklib_stubs.c — placeholder bodies for the ZisK assembly library C bindings.
 *
 * Each function here is a real, un-inlined, exported symbol whose entry is
 * redirected to the matching hand-written zisklib_* .zisk routine by elf2rom at
 * transpile time (see include/zisklib.h and elf2rom's REDIRECTS table). The
 * bodies only run in a non-transpiled build; in the ZisK guest they are never
 * executed — the .zisk routine runs instead.
 *
 * Two properties must hold for the redirect to work, mirroring the Rust binding:
 *   1. Stable exported symbol with a real size — hence `noinline` + `used`, and
 *      external linkage (never `static`). Keep the ELF's .symtab (do not strip):
 *      elf2rom looks the symbols up by name.
 *   2. Every argument must be materialized at the call site. The redirected
 *      routine reads its arguments from a0..a7; a body that ignored an argument
 *      could let the optimizer skip setting up that register. TOUCH() forces each
 *      argument into a register and blocks that optimization.
 *
 * The placeholder results are argument-dependent sentinels: if the redirect did
 * not fire, callers observe an obviously-wrong value rather than a plausible one.
 */

#include "zisklib.h"

#define ZK_STUB __attribute__((noinline, used))

/* Force `x` to be materialized in a register; prevents the optimizer from
 * eliding the corresponding a0..a7 setup at the call site. */
#define TOUCH(x) __asm__ volatile("" : : "r"(x) : "memory")

/* Write a sentinel byte pattern into an output buffer so the stub has a real,
 * side-effecting body (and a nonzero size in the symbol table). */
static inline void zk_fill(void *p, unsigned char b, size_t n) {
    volatile unsigned char *q = (volatile unsigned char *)p;
    for (size_t i = 0; i < n; ++i) q[i] = b;
}

/* ---- demo -------------------------------------------------------------- */

ZK_STUB uint64_t ziskos_add(uint64_t a, uint64_t b) {
    TOUCH(a); TOUCH(b);
    return 0xBAD00000000ULL + a + b;
}

/* ---- hashing ----------------------------------------------------------- */

ZK_STUB void ziskos_keccak(const uint8_t *input, size_t len, uint8_t *output) {
    TOUCH(input); TOUCH(len); TOUCH(output);
    zk_fill(output, 0xBA, 32);
}

ZK_STUB void ziskos_sha256(const uint8_t *input, size_t len, uint8_t *output) {
    TOUCH(input); TOUCH(len); TOUCH(output);
    zk_fill(output, 0xB5, 32);
}

ZK_STUB void ziskos_blake2b_compress(uint32_t rounds, uint64_t *state,
                                     const uint64_t *message, const uint64_t *offset,
                                     uint8_t final_block) {
    TOUCH(rounds); TOUCH(state); TOUCH(message); TOUCH(offset); TOUCH(final_block);
    zk_fill(state, 0xB2, 8 * sizeof(uint64_t));
}

/* ---- 256-bit integer arithmetic ---------------------------------------- */

ZK_STUB uint64_t ziskos_inv256(const uint64_t *a, uint64_t *result) {
    TOUCH(a); TOUCH(result);
    zk_fill(result, 0xC0, 4 * sizeof(uint64_t));
    return 0xBAD256ULL ^ (uint64_t)(uintptr_t)a;
}

ZK_STUB uint64_t ziskos_overflowing_add256(const uint64_t *a, const uint64_t *b, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    zk_fill(result, 0xC1, 4 * sizeof(uint64_t));
    return 1;
}
ZK_STUB uint64_t ziskos_overflowing_sub256(const uint64_t *a, const uint64_t *b, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    zk_fill(result, 0xC2, 4 * sizeof(uint64_t));
    return 1;
}
ZK_STUB uint64_t ziskos_overflowing_mul256(const uint64_t *a, const uint64_t *b, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    zk_fill(result, 0xC3, 4 * sizeof(uint64_t));
    return 1;
}

ZK_STUB void ziskos_div_rem256(const uint64_t *a, const uint64_t *b, uint64_t *q, uint64_t *r) {
    TOUCH(a); TOUCH(b); TOUCH(q); TOUCH(r);
    zk_fill(q, 0xC4, 4 * sizeof(uint64_t));
    zk_fill(r, 0xC5, 4 * sizeof(uint64_t));
}

ZK_STUB void ziskos_reduce_mod256(const uint64_t *a, const uint64_t *m, uint64_t *result) {
    TOUCH(a); TOUCH(m); TOUCH(result);
    zk_fill(result, 0xC6, 4 * sizeof(uint64_t));
}
ZK_STUB void ziskos_add_mod256(const uint64_t *a, const uint64_t *b, const uint64_t *m, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(m); TOUCH(result);
    zk_fill(result, 0xC7, 4 * sizeof(uint64_t));
}
ZK_STUB void ziskos_mul_mod256(const uint64_t *a, const uint64_t *b, const uint64_t *m, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(m); TOUCH(result);
    zk_fill(result, 0xC8, 4 * sizeof(uint64_t));
}
ZK_STUB uint64_t ziskos_inv_mod256(const uint64_t *a, const uint64_t *m, uint64_t *result) {
    TOUCH(a); TOUCH(m); TOUCH(result);
    zk_fill(result, 0xC9, 4 * sizeof(uint64_t));
    return 0xBAD30D256ULL ^ (uint64_t)(uintptr_t)a;
}
ZK_STUB void ziskos_pow_mod256(const uint64_t *base, const uint64_t *exp, const uint64_t *m, uint64_t *result) {
    TOUCH(base); TOUCH(exp); TOUCH(m); TOUCH(result);
    zk_fill(result, 0xCA, 4 * sizeof(uint64_t));
}
ZK_STUB uint64_t ziskos_overflowing_pow256(const uint64_t *base, const uint64_t *exp, uint64_t *result) {
    TOUCH(base); TOUCH(exp); TOUCH(result);
    zk_fill(result, 0xCB, 4 * sizeof(uint64_t));
    return 1;
}

/* ---- secp256k1 --------------------------------------------------------- */

ZK_STUB uint64_t ziskos_ecdsa_verify_secp256k1(const uint64_t *pk, const uint64_t *z,
                                               const uint64_t *r, const uint64_t *s) {
    TOUCH(pk); TOUCH(z); TOUCH(r); TOUCH(s);
    return 0xBADEC1ULL ^ (uint64_t)(uintptr_t)pk;
}
ZK_STUB uint64_t ziskos_ecdsa_recover_secp256k1(const uint64_t *r, const uint64_t *s,
                                                const uint64_t *z, uint64_t recid,
                                                uint64_t *result) {
    TOUCH(r); TOUCH(s); TOUCH(z); TOUCH(recid); TOUCH(result);
    zk_fill(result, 0xE0, 8 * sizeof(uint64_t));
    return 0xBADEC2ULL ^ recid;
}
ZK_STUB uint64_t ziskos_schnorr_verify_secp256k1(const uint64_t *pk_x, const uint64_t *r,
                                                 const uint64_t *s, const uint8_t *msg,
                                                 uint64_t msg_len) {
    TOUCH(pk_x); TOUCH(r); TOUCH(s); TOUCH(msg); TOUCH(msg_len);
    return 0xBADEC3ULL ^ msg_len;
}

/* ---- secp256r1 --------------------------------------------------------- */

ZK_STUB uint64_t ziskos_ecdsa_verify_secp256r1(const uint64_t *pk, const uint64_t *z,
                                               const uint64_t *r, const uint64_t *s) {
    TOUCH(pk); TOUCH(z); TOUCH(r); TOUCH(s);
    return 0xBAD256E1ULL ^ (uint64_t)(uintptr_t)pk;
}

/* ---- BN254 ------------------------------------------------------------- */

ZK_STUB uint64_t ziskos_pairing_check_bn254(const uint64_t *g1, const uint64_t *g2, uint64_t n) {
    TOUCH(g1); TOUCH(g2); TOUCH(n);
    return 0x0BADB254ULL ^ n;
}

/* ---- BLS12-381 --------------------------------------------------------- */

ZK_STUB uint64_t ziskos_pairing_check_bls12_381(const uint64_t *g1, const uint64_t *g2, uint64_t n) {
    TOUCH(g1); TOUCH(g2); TOUCH(n);
    return 0x0BADB157ULL ^ n;
}
ZK_STUB uint64_t ziskos_map_to_curve_g1_bls12_381(const uint64_t *u, uint64_t *result) {
    TOUCH(u); TOUCH(result);
    zk_fill(result, 0xA1, 12 * sizeof(uint64_t));
    return 0x0BADA11CULL ^ (uint64_t)(uintptr_t)u;
}
ZK_STUB uint64_t ziskos_map_to_curve_g2_bls12_381(const uint64_t *u, uint64_t *result) {
    TOUCH(u); TOUCH(result);
    zk_fill(result, 0xA2, 24 * sizeof(uint64_t));
    return 0x0BADA22CULL ^ (uint64_t)(uintptr_t)u;
}
ZK_STUB void ziskos_hash_to_curve_g2_bls12_381(const uint8_t *msg, uint64_t msg_len,
                                               const uint8_t *dst, uint64_t dst_len,
                                               uint64_t *result) {
    TOUCH(msg); TOUCH(msg_len); TOUCH(dst); TOUCH(dst_len); TOUCH(result);
    zk_fill(result, 0xA3, 24 * sizeof(uint64_t));
}
ZK_STUB uint64_t ziskos_bls_verify_bls12_381(const uint8_t *pk, const uint8_t *msg,
                                             uint64_t msg_len, const uint8_t *sig) {
    TOUCH(pk); TOUCH(msg); TOUCH(msg_len); TOUCH(sig);
    return 0x0BADB15ULL ^ msg_len;
}
ZK_STUB uint64_t ziskos_verify_kzg_proof_bls12_381(const uint8_t *z, const uint8_t *y,
                                                   const uint8_t *commitment,
                                                   const uint8_t *proof) {
    TOUCH(z); TOUCH(y); TOUCH(commitment); TOUCH(proof);
    return 0x0BAD4844ULL ^ (uint64_t)(uintptr_t)z;
}

/* ---- bigint / MODEXP --------------------------------------------------- */

ZK_STUB size_t ziskos_modexp_u64_c(const uint64_t *base, size_t base_len,
                                   const uint64_t *exp, size_t exp_len,
                                   const uint64_t *modulus, size_t modulus_len,
                                   uint64_t *result) {
    TOUCH(base); TOUCH(base_len); TOUCH(exp); TOUCH(exp_len);
    TOUCH(modulus); TOUCH(modulus_len); TOUCH(result);
    zk_fill(result, 0xE1, 4 * sizeof(uint64_t));
    return 0x0BADE198ULL ^ base_len ^ exp_len ^ modulus_len;
}
