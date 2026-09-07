/*
 * zkvm_stubs.c — C stubs for the Ethereum Foundation zkVM accelerator ABI
 * (zkvm_accelerators.h). Each `zkvm_*` function is a raw-ABI stub with a stable,
 * un-mangled symbol and a placeholder body; during transpilation elf2rom
 * redirects its entry DIRECTLY to the matching native `ziskasm_zkvm_*` routine
 * under ziskasm/zisklib/zkvm/. There is NO wrapper and NO intermediate ziskos_*
 * layer: a guest that follows the EF standard calls `zkvm_xxx()` and, in one
 * redirected call, runs the hand-written ziskasm crypto.
 *
 * A guest links EITHER these stubs (the ZisK-accelerated path) OR the portable
 * `zkvm-interface` Rust/C implementation of the same standard symbols — never
 * both; the two are mutually-exclusive definitions of the EF ABI.
 *
 * The placeholder bodies are NEVER meant to run: elf2rom skips them. If one does
 * run, the redirect did not fire (a stripped ELF, a missing symbol, or ziskemu/
 * cargo-zisk built without the `ziskasm` feature), so the stub FAILS HARD — it
 * prints a diagnostic and faults on the null guard page — rather than returning a
 * plausible-but-wrong value that could be mistaken for a real result.
 *
 * Stub rules that keep the redirect working (mirrors src/zisklib_stubs.c):
 *   - noinline + used, external linkage: a stable symbol with a real size.
 *   - every argument is touched, so a0..a7 are materialised at the call site for
 *     the redirected routine to read.
 *   - do NOT --strip the guest ELF: elf2rom resolves the stubs by name.
 */
#include "zkvm_accelerators.h"
#include "zkvm_u256.h"

#define ZKVM_STUB __attribute__((noinline, used))
#define TOUCH(x)  __asm__ volatile("" : : "r"(x) : "memory")

/* Emit a one-line diagnostic to the ZisK memory-mapped stdout (UART at
 * 0xA0400200, one byte per store), then access the null guard page (address 0)
 * to force abnormal termination. `noreturn`: it never comes back, so the callers
 * need no return value. Reached only when the elf2rom redirect did not fire. */
__attribute__((noinline, noreturn))
static void zkvm_stub_fail(const char *fn) {
    static const char pre[]  = "ERROR: ziskasm zkVM stub reached without redirect: ";
    static const char post[] = "() -- build ziskemu/cargo-zisk with --features ziskasm "
                               "and do not strip the guest ELF\n";
    volatile uint8_t *const uart = (volatile uint8_t *)0xA0400200ULL;  /* ZisK stdout */
    for (const char *p = pre;  *p; ++p) *uart = (uint8_t)*p;
    for (const char *p = fn;   *p; ++p) *uart = (uint8_t)*p;
    for (const char *p = post; *p; ++p) *uart = (uint8_t)*p;
    volatile uintptr_t null_addr = 0;   /* volatile: force a real access, not folded away */
    *(volatile uint8_t *)null_addr = 0; /* touch address 0 -> abnormal termination */
    __builtin_unreachable();
}
#define STUB_FAIL()  zkvm_stub_fail(__func__)

/* ---- hashes (byte-in / byte-out; no marshalling) ----------------------- */
ZKVM_STUB zkvm_status zkvm_keccak256(const uint8_t *data, size_t len,
                                     zkvm_keccak256_hash *output) {
    TOUCH(data); TOUCH(len); TOUCH(output);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_sha256(const uint8_t *data, size_t len,
                                  zkvm_sha256_hash *output) {
    TOUCH(data); TOUCH(len); TOUCH(output);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_keccak_f1600(uint64_t *state) {   /* 25-word state, in place */
    TOUCH(state);
    STUB_FAIL();
}

/* ---- secp256k1 --------------------------------------------------------- */
ZKVM_STUB zkvm_status zkvm_secp256k1_verify(const zkvm_secp256k1_hash *msg,
                                            const zkvm_secp256k1_signature *sig,
                                            const zkvm_secp256k1_pubkey *pubkey,
                                            bool *verified) {
    TOUCH(msg); TOUCH(sig); TOUCH(pubkey); TOUCH(verified);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_secp256k1_ecrecover(const zkvm_secp256k1_hash *msg,
                                               const zkvm_secp256k1_signature *sig,
                                               uint8_t recid,
                                               zkvm_secp256k1_pubkey *output) {
    TOUCH(msg); TOUCH(sig); TOUCH(recid); TOUCH(output);
    STUB_FAIL();
}

/* ---- secp256r1 --------------------------------------------------------- */
ZKVM_STUB zkvm_status zkvm_secp256r1_verify(const zkvm_secp256r1_hash *msg,
                                            const zkvm_secp256r1_signature *sig,
                                            const zkvm_secp256r1_pubkey *pubkey,
                                            bool *verified) {
    TOUCH(msg); TOUCH(sig); TOUCH(pubkey); TOUCH(verified);
    STUB_FAIL();
}

/* ---- blake2f ----------------------------------------------------------- */
ZKVM_STUB zkvm_status zkvm_blake2f(uint32_t rounds, zkvm_blake2f_state *h,
                                   const zkvm_blake2f_message *m,
                                   const zkvm_blake2f_offset *t, uint8_t f) {
    TOUCH(rounds); TOUCH(h); TOUCH(m); TOUCH(t); TOUCH(f);
    STUB_FAIL();
}

/* ---- modexp (EIP-198; arbitrary-length big-endian byte operands) -------- */
ZKVM_STUB zkvm_status zkvm_modexp(const uint8_t *base, size_t base_len,
                                  const uint8_t *exp, size_t exp_len,
                                  const uint8_t *modulus, size_t mod_len,
                                  uint8_t *output) {
    TOUCH(base); TOUCH(base_len); TOUCH(exp); TOUCH(exp_len);
    TOUCH(modulus); TOUCH(mod_len); TOUCH(output);
    STUB_FAIL();
}

/* ---- BN254 (alt_bn128) ------------------------------------------------- */
ZKVM_STUB zkvm_status zkvm_bn254_g1_add(const zkvm_bn254_g1_point *p1,
                                        const zkvm_bn254_g1_point *p2,
                                        zkvm_bn254_g1_point *result) {
    TOUCH(p1); TOUCH(p2); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_bn254_g1_mul(const zkvm_bn254_g1_point *point,
                                        const zkvm_bn254_scalar *scalar,
                                        zkvm_bn254_g1_point *result) {
    TOUCH(point); TOUCH(scalar); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_bn254_pairing(const zkvm_bn254_pairing_pair *pairs,
                                         size_t num_pairs, bool *verified) {
    TOUCH(pairs); TOUCH(num_pairs); TOUCH(verified);
    STUB_FAIL();
}

/* ---- BLS12-381 (EIP-2537) + KZG (EIP-4844) ----------------------------- */
ZKVM_STUB zkvm_status zkvm_bls12_g1_add(const zkvm_bls12_381_g1_point *p1,
                                        const zkvm_bls12_381_g1_point *p2,
                                        zkvm_bls12_381_g1_point *result) {
    TOUCH(p1); TOUCH(p2); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_bls12_g1_msm(const zkvm_bls12_381_g1_msm_pair *pairs,
                                        size_t num_pairs, zkvm_bls12_381_g1_point *result) {
    TOUCH(pairs); TOUCH(num_pairs); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_bls12_g2_add(const zkvm_bls12_381_g2_point *p1,
                                        const zkvm_bls12_381_g2_point *p2,
                                        zkvm_bls12_381_g2_point *result) {
    TOUCH(p1); TOUCH(p2); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_bls12_g2_msm(const zkvm_bls12_381_g2_msm_pair *pairs,
                                        size_t num_pairs, zkvm_bls12_381_g2_point *result) {
    TOUCH(pairs); TOUCH(num_pairs); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_bls12_pairing(const zkvm_bls12_381_pairing_pair *pairs,
                                         size_t num_pairs, bool *verified) {
    TOUCH(pairs); TOUCH(num_pairs); TOUCH(verified);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_bls12_map_fp_to_g1(const zkvm_bls12_381_fp *field_element,
                                              zkvm_bls12_381_g1_point *result) {
    TOUCH(field_element); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_bls12_map_fp2_to_g2(const zkvm_bls12_381_fp2 *field_element,
                                               zkvm_bls12_381_g2_point *result) {
    TOUCH(field_element); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_kzg_point_eval(const zkvm_kzg_commitment *commitment,
                                          const zkvm_kzg_field_element *z,
                                          const zkvm_kzg_field_element *y,
                                          const zkvm_kzg_proof *proof, bool *verified) {
    TOUCH(commitment); TOUCH(z); TOUCH(y); TOUCH(proof); TOUCH(verified);
    STUB_FAIL();
}

/* ---- RIPEMD-160 (byte-in; 20-byte digest right-aligned in 32-byte output) --- */
ZKVM_STUB zkvm_status zkvm_ripemd160(const uint8_t *data, size_t len,
                                     zkvm_ripemd160_hash *output) {
    TOUCH(data); TOUCH(len); TOUCH(output);
    STUB_FAIL();
}

/* ---- U256 EVM-word arithmetic (zkvm_u256.h; big-endian 32-byte operands) ---- */
ZKVM_STUB zkvm_status zkvm_u256_add(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_sub(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_mul(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_div(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *quotient) {
    TOUCH(a); TOUCH(b); TOUCH(quotient);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_mod(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *remainder) {
    TOUCH(a); TOUCH(b); TOUCH(remainder);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_divmod(const zkvm_u256 *a, const zkvm_u256 *b,
                                       zkvm_u256 *quotient, zkvm_u256 *remainder) {
    TOUCH(a); TOUCH(b); TOUCH(quotient); TOUCH(remainder);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_addmod(const zkvm_u256 *a, const zkvm_u256 *b,
                                       const zkvm_u256 *n, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(n); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_mulmod(const zkvm_u256 *a, const zkvm_u256 *b,
                                       const zkvm_u256 *n, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(n); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_exp(const zkvm_u256 *base, const zkvm_u256 *exponent,
                                    zkvm_u256 *result) {
    TOUCH(base); TOUCH(exponent); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_sdiv(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *quotient) {
    TOUCH(a); TOUCH(b); TOUCH(quotient);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_smod(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *remainder) {
    TOUCH(a); TOUCH(b); TOUCH(remainder);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_sdivmod(const zkvm_u256 *a, const zkvm_u256 *b,
                                        zkvm_u256 *quotient, zkvm_u256 *remainder) {
    TOUCH(a); TOUCH(b); TOUCH(quotient); TOUCH(remainder);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_lt(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_gt(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_slt(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_sgt(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_eq(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_iszero(const zkvm_u256 *a, zkvm_u256 *result) {
    TOUCH(a); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_and(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_or(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_xor(const zkvm_u256 *a, const zkvm_u256 *b, zkvm_u256 *result) {
    TOUCH(a); TOUCH(b); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_not(const zkvm_u256 *a, zkvm_u256 *result) {
    TOUCH(a); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_byte(const zkvm_u256 *i, const zkvm_u256 *a, zkvm_u256 *result) {
    TOUCH(i); TOUCH(a); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_shl(const zkvm_u256 *shift, const zkvm_u256 *value, zkvm_u256 *result) {
    TOUCH(shift); TOUCH(value); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_shr(const zkvm_u256 *shift, const zkvm_u256 *value, zkvm_u256 *result) {
    TOUCH(shift); TOUCH(value); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_sar(const zkvm_u256 *shift, const zkvm_u256 *value, zkvm_u256 *result) {
    TOUCH(shift); TOUCH(value); TOUCH(result);
    STUB_FAIL();
}
ZKVM_STUB zkvm_status zkvm_u256_signextend(const zkvm_u256 *b, const zkvm_u256 *value, zkvm_u256 *result) {
    TOUCH(b); TOUCH(value); TOUCH(result);
    STUB_FAIL();
}
