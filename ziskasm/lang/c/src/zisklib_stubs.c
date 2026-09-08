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
 * If a placeholder body ever runs, the redirect did NOT fire (a stripped ELF, a
 * missing symbol, or ziskemu/cargo-zisk built without the `ziskasm` feature), so
 * the stub FAILS HARD — it prints a diagnostic and faults on the null guard page
 * (address 0) — rather than returning a plausible-but-wrong value.
 */

#include "zisklib.h"

#define ZK_STUB __attribute__((noinline, used))

/* Force `x` to be materialized in a register; prevents the optimizer from
 * eliding the corresponding a0..a7 setup at the call site. */
#define TOUCH(x) __asm__ volatile("" : : "r"(x) : "memory")

/* Emit a one-line diagnostic to the ZisK memory-mapped stdout (UART at
 * 0xA0400200, one byte per store), then access the null guard page (address 0)
 * to force abnormal termination. Reached only when the elf2rom redirect did not
 * fire.
 *
 * IMPORTANT: this is deliberately NOT `noreturn`. The volatile store to address 0
 * aborts the machine at runtime, but to the compiler it is an ordinary returning
 * function (a store, not a diverge). Were it `noreturn`, every stub that ends in
 * it would be inferred `noreturn`, and any caller that can see the body -- under
 * LTO, or if a stub shares a translation unit with a caller -- would delete its
 * own code AFTER the call. Since the redirected `.zisk` routine returns normally,
 * that would corrupt the guest. So value-returning stubs `return
 * zisklib_stub_fail(..)` (STUB_FAIL) and void stubs call it as a statement
 * (STUB_FAIL_VOID); either way the stub stays non-diverging and no caller is ever
 * miscompiled. The returned value is never reached at runtime. */
__attribute__((noinline))
static uint64_t zisklib_stub_fail(const char *fn) {
    static const char pre[]  = "ERROR: ziskasm library stub reached without redirect: ";
    static const char post[] = "() -- build ziskemu/cargo-zisk with --features ziskasm "
                               "and do not strip the guest ELF\n";
    volatile uint8_t *const uart = (volatile uint8_t *)0xA0400200ULL;  /* ZisK stdout */
    for (const char *p = pre;  *p; ++p) *uart = (uint8_t)*p;
    for (const char *p = fn;   *p; ++p) *uart = (uint8_t)*p;
    for (const char *p = post; *p; ++p) *uart = (uint8_t)*p;
    volatile uintptr_t null_addr = 0;   /* volatile: force a real access, not folded away */
    *(volatile uint8_t *)null_addr = 0; /* touch address 0 -> abnormal termination */
    return 0;                           /* never reached at runtime (faulted above) */
}
#define STUB_FAIL()       return zisklib_stub_fail(__func__)  /* value-returning stubs */
#define STUB_FAIL_VOID()  (void)zisklib_stub_fail(__func__)   /* void stubs */

/* ---- demo -------------------------------------------------------------- */

ZK_STUB uint64_t ziskos_add(uint64_t a, uint64_t b) {
    TOUCH(a); TOUCH(b);
    STUB_FAIL();
}

/* ---- hashing ----------------------------------------------------------- */

ZK_STUB void ziskos_keccak(const uint8_t *input, size_t len, uint8_t *output) {
    TOUCH(input); TOUCH(len); TOUCH(output);
    STUB_FAIL_VOID();
}

ZK_STUB void ziskos_sha256(const uint8_t *input, size_t len, uint8_t *output) {
    TOUCH(input); TOUCH(len); TOUCH(output);
    STUB_FAIL_VOID();
}

ZK_STUB void ziskos_blake2b_compress(uint32_t rounds, uint64_t *state,
                                     const uint64_t *message, const uint64_t *offset,
                                     uint8_t final_block) {
    TOUCH(rounds); TOUCH(state); TOUCH(message); TOUCH(offset); TOUCH(final_block);
    STUB_FAIL_VOID();
}

/* ---- 256-bit integer arithmetic ---------------------------------------- */

ZK_STUB uint64_t ziskos_inv256(const uint64_t *a, uint64_t *result) {
    TOUCH(a); TOUCH(result);
    STUB_FAIL();
}

ZK_STUB uint64_t ziskos_overflowing_add256(const uint64_t *a, const uint64_t *b, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZK_STUB uint64_t ziskos_overflowing_sub256(const uint64_t *a, const uint64_t *b, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZK_STUB uint64_t ziskos_overflowing_mul256(const uint64_t *a, const uint64_t *b, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}

ZK_STUB void ziskos_div_rem256(const uint64_t *a, const uint64_t *b, uint64_t *q, uint64_t *r) {
    TOUCH(a); TOUCH(b); TOUCH(q); TOUCH(r);
    STUB_FAIL_VOID();
}

ZK_STUB void ziskos_reduce_mod256(const uint64_t *a, const uint64_t *m, uint64_t *result) {
    TOUCH(a); TOUCH(m); TOUCH(result);
    STUB_FAIL_VOID();
}
ZK_STUB void ziskos_add_mod256(const uint64_t *a, const uint64_t *b, const uint64_t *m, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(m); TOUCH(result);
    STUB_FAIL_VOID();
}
ZK_STUB void ziskos_mul_mod256(const uint64_t *a, const uint64_t *b, const uint64_t *m, uint64_t *result) {
    TOUCH(a); TOUCH(b); TOUCH(m); TOUCH(result);
    STUB_FAIL_VOID();
}
ZK_STUB uint64_t ziskos_inv_mod256(const uint64_t *a, const uint64_t *m, uint64_t *result) {
    TOUCH(a); TOUCH(m); TOUCH(result);
    STUB_FAIL();
}
ZK_STUB void ziskos_pow_mod256(const uint64_t *base, const uint64_t *exp, const uint64_t *m, uint64_t *result) {
    TOUCH(base); TOUCH(exp); TOUCH(m); TOUCH(result);
    STUB_FAIL_VOID();
}
ZK_STUB uint64_t ziskos_overflowing_pow256(const uint64_t *base, const uint64_t *exp, uint64_t *result) {
    TOUCH(base); TOUCH(exp); TOUCH(result);
    STUB_FAIL();
}

/* ---- secp256k1 --------------------------------------------------------- */

ZK_STUB uint64_t ziskos_ecdsa_verify_secp256k1(const uint64_t *pk, const uint64_t *z,
                                               const uint64_t *r, const uint64_t *s) {
    TOUCH(pk); TOUCH(z); TOUCH(r); TOUCH(s);
    STUB_FAIL();
}
ZK_STUB uint64_t ziskos_ecdsa_recover_secp256k1(const uint64_t *r, const uint64_t *s,
                                                const uint64_t *z, uint64_t recid,
                                                uint64_t *result) {
    TOUCH(r); TOUCH(s); TOUCH(z); TOUCH(recid); TOUCH(result);
    STUB_FAIL();
}
ZK_STUB uint64_t ziskos_schnorr_verify_secp256k1(const uint64_t *pk_x, const uint64_t *r,
                                                 const uint64_t *s, const uint8_t *msg,
                                                 uint64_t msg_len) {
    TOUCH(pk_x); TOUCH(r); TOUCH(s); TOUCH(msg); TOUCH(msg_len);
    STUB_FAIL();
}

/* ---- secp256r1 --------------------------------------------------------- */

ZK_STUB uint64_t ziskos_ecdsa_verify_secp256r1(const uint64_t *pk, const uint64_t *z,
                                               const uint64_t *r, const uint64_t *s) {
    TOUCH(pk); TOUCH(z); TOUCH(r); TOUCH(s);
    STUB_FAIL();
}

/* ---- BN254 ------------------------------------------------------------- */

ZK_STUB uint64_t ziskos_pairing_check_bn254(const uint64_t *g1, const uint64_t *g2, uint64_t n) {
    TOUCH(g1); TOUCH(g2); TOUCH(n);
    STUB_FAIL();
}

/* ---- BLS12-381 --------------------------------------------------------- */

ZK_STUB uint64_t ziskos_pairing_check_bls12_381(const uint64_t *g1, const uint64_t *g2, uint64_t n) {
    TOUCH(g1); TOUCH(g2); TOUCH(n);
    STUB_FAIL();
}
ZK_STUB uint64_t ziskos_map_to_curve_g1_bls12_381(const uint64_t *u, uint64_t *result) {
    TOUCH(u); TOUCH(result);
    STUB_FAIL();
}
ZK_STUB uint64_t ziskos_map_to_curve_g2_bls12_381(const uint64_t *u, uint64_t *result) {
    TOUCH(u); TOUCH(result);
    STUB_FAIL();
}
ZK_STUB void ziskos_hash_to_curve_g2_bls12_381(const uint8_t *msg, uint64_t msg_len,
                                               const uint8_t *dst, uint64_t dst_len,
                                               uint64_t *result) {
    TOUCH(msg); TOUCH(msg_len); TOUCH(dst); TOUCH(dst_len); TOUCH(result);
    STUB_FAIL_VOID();
}
ZK_STUB uint64_t ziskos_bls_verify_bls12_381(const uint8_t *pk, const uint8_t *msg,
                                             uint64_t msg_len, const uint8_t *sig) {
    TOUCH(pk); TOUCH(msg); TOUCH(msg_len); TOUCH(sig);
    STUB_FAIL();
}
ZK_STUB uint64_t ziskos_verify_kzg_proof_bls12_381(const uint8_t *z, const uint8_t *y,
                                                   const uint8_t *commitment,
                                                   const uint8_t *proof) {
    TOUCH(z); TOUCH(y); TOUCH(commitment); TOUCH(proof);
    STUB_FAIL();
}

/* ---- bigint / MODEXP --------------------------------------------------- */

ZK_STUB size_t ziskos_modexp_u64_c(const uint64_t *base, size_t base_len,
                                   const uint64_t *exp, size_t exp_len,
                                   const uint64_t *modulus, size_t modulus_len,
                                   uint64_t *result) {
    TOUCH(base); TOUCH(base_len); TOUCH(exp); TOUCH(exp_len);
    TOUCH(modulus); TOUCH(modulus_len); TOUCH(result);
    STUB_FAIL();
}
