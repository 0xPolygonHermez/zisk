/* zisklib_calls.c -- every function of zkvm_zisklib.h (zkvmcall thunks), called on
   fixed inputs: checks the zkvmcall plumbing (the arguments in a0..a7, the return
   value and the result buffers) of each. The golden values were first checked
   against the same routines reached through the former symbol redirection.
   Output: per call, the 8-byte FNV-1a hash of its return value and results.
   check.sh compares it with a golden snapshot. */
#include <stddef.h>
#include <stdint.h>

#include "zkvm_zisklib.h"

static volatile uint8_t* const O = (volatile uint8_t*)0xA0410000ULL;

static uint64_t h;
static void mix(const void* p, size_t n) {
    const uint8_t* b = (const uint8_t*)p;
    for (size_t i = 0; i < n; i++) { h ^= b[i]; h *= 0x100000001b3ULL; }
}
static void mixv(uint64_t v) { mix(&v, 8); }

/* secp256k1 ECDSA vector (test-artifacts secp256k1 ecdsa test), little-endian limbs */
static const uint64_t PK[8] = {
    0x3bcfdc2aca47e0f2, 0xa739d5cc6b89e9b5, 0x35b73cc431afc6bc, 0xe1ea4273f638d4ae,
    0xc6402318ee33448e, 0x9f18c242b8df8bb6, 0x934a8dfdd797e1c4, 0x3840aa9c4d86557e};
static const uint64_t Z[4] = {0x1bf86a1816a52f52, 0xd31e26c3da73dda8, 0xa3b71997594da038, 0x17560495f6944673};
static const uint64_t R[4] = {0x68df7d8d7e0fb36b, 0xc2189fe681cd6e78, 0xc85ba1fd6238ecb5, 0x3e125456c8338994};
static const uint64_t S[4] = {0xd4e89d1ae75aeea2, 0xb8e33178783bd1a3, 0x0866acebc9e141ec, 0x3a816b1c33739e41};

static const uint64_t A[4] = {0x0123456789abcdef, 0xfedcba9876543210, 0x0f1e2d3c4b5a6978, 0x1122334455667788};
static const uint64_t B[4] = {0x8877665544332211, 0x0011223344556677, 0x99aabbccddeeff00, 0x0000000000000abc};
static const uint64_t M[4] = {0xffffffff00000001, 0x0000000000000000, 0x00000000ffffffff, 0xffffffff00000000};
static const uint64_t E[4] = {0x10001, 0, 0, 0};
/* map_to_curve inputs: an Fp element (6 limbs) and an Fp2 element (12 limbs), each below p */
static const uint64_t U_FP[6] = {1, 0, 0, 0, 0, 0};
static const uint64_t U_FP2[12] = {0x0123456789abcdef, 0xfedcba9876543210, 0x0f1e2d3c4b5a6978,
                                   0x1122334455667788, 0, 0, 2, 0, 0, 0, 0, 0};

static const uint8_t MSG[3] = {'a', 'b', 'c'};
static const uint8_t DST[] = "QUUX-V01-CS02-with-BLS12381G2_XMD:SHA-256_SSWU_RO_";
static const uint8_t K32[32] = {1, 2, 3}, K48[48] = {0xc0}, K96[96] = {0xc0};

/* One call: body(DIRECT) into `outbuf`, hashed with its return value. */
#define DIRECT(n) zkvm_zisklib_##n
#define CALL(body, outbuf, outlen)                                                 \
    do {                                                                           \
        static uint64_t oa[32];                                                    \
        for (int i = 0; i < 32; i++) oa[i] = 0x5a5a5a5a5a5a5a5aULL;                \
        uint64_t ra;                                                               \
        { uint64_t* outbuf = oa; ra = (uint64_t)(body(DIRECT)); }                  \
        h = 0xcbf29ce484222325ULL; mixv(ra); mix(oa, outlen);                      \
        for (int i = 0; i < 8; i++) O[k++] = (uint8_t)(h >> (8 * i));              \
    } while (0)

int main(void) {
    unsigned k = 0;
    static uint64_t q[4], r2[4], g1[12], g2[24], big[16];
#define F_ADD(p) p(add)(0x1234, 0x5678) + (out[0] = 0)
    CALL(F_ADD, out, 8);
#define F_INV(p) p(inv256)(A, out)
    CALL(F_INV, out, 32);
#define F_OADD(p) p(overflowing_add256)(A, B, out)
    CALL(F_OADD, out, 32);
#define F_OSUB(p) p(overflowing_sub256)(B, A, out)
    CALL(F_OSUB, out, 32);
#define F_OMUL(p) p(overflowing_mul256)(A, B, out)
    CALL(F_OMUL, out, 32);
#define F_DIVREM(p) (p(div_rem256)(A, B, out, out + 4), 0)
    CALL(F_DIVREM, out, 64);
#define F_RED(p) (p(reduce_mod256)(A, M, out), 0)
    CALL(F_RED, out, 32);
#define F_ADDM(p) (p(add_mod256)(A, B, M, out), 0)
    CALL(F_ADDM, out, 32);
#define F_MULM(p) (p(mul_mod256)(A, B, M, out), 0)
    CALL(F_MULM, out, 32);
#define F_INVM(p) p(inv_mod256)(B, M, out)
    CALL(F_INVM, out, 32);
#define F_POWM(p) (p(pow_mod256)(A, E, M, out), 0)
    CALL(F_POWM, out, 32);
#define F_OPOW(p) p(overflowing_pow256)(B, E, out)
    CALL(F_OPOW, out, 32);
#define F_K1V(p) p(ecdsa_verify_secp256k1)(PK, Z, R, S) + (out[0] = 0)
    CALL(F_K1V, out, 8);
#define F_K1R(p) p(ecdsa_recover_secp256k1)(R, S, Z, 1, out)
    CALL(F_K1R, out, 64);
#define F_SCH(p) p(schnorr_verify_secp256k1)(PK, R, S, MSG, 3) + (out[0] = 0)
    CALL(F_SCH, out, 8);
#define F_R1V(p) p(ecdsa_verify_secp256r1)(PK, Z, R, S) + (out[0] = 0)
    CALL(F_R1V, out, 8);
#define F_BNP0(p) p(pairing_check_bn254)(g1, g2, 0) + (out[0] = 0)
    CALL(F_BNP0, out, 8);
#define F_BNP1(p) p(pairing_check_bn254)(PK, big, 1) + (out[0] = 0)
    CALL(F_BNP1, out, 8);
#define F_BLP(p) p(pairing_check_bls12_381)(g1, g2, 0) + (out[0] = 0)
    CALL(F_BLP, out, 8);
#define F_MAP1(p) p(map_to_curve_g1_bls12_381)(U_FP, out)
    CALL(F_MAP1, out, 96);
#define F_MAP2(p) p(map_to_curve_g2_bls12_381)(U_FP2, out)
    CALL(F_MAP2, out, 192);
#define F_H2C(p) (p(hash_to_curve_g2_bls12_381)(MSG, 3, DST, sizeof DST - 1, out), 0)
    CALL(F_H2C, out, 192);
#define F_BLSV(p) p(bls_verify_bls12_381)(K48, MSG, 3, K96) + (out[0] = 0)
    CALL(F_BLSV, out, 8);
#define F_KZG(p) p(verify_kzg_proof_bls12_381)(K32, K32, K48, K48) + (out[0] = 0)
    CALL(F_KZG, out, 8);
#define F_MX(p) p(modexp_u64_c)(A, 4, E, 1, M, 4, out)
    CALL(F_MX, out, 32);
    (void)q; (void)r2;
    return 0;
}
