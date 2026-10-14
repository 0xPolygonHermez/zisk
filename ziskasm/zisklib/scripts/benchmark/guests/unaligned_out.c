/* unaligned_out.c -- ZisK library and zkvm_* functions called with their result
   buffer at byte offsets 0..7. The precompiles need 8-byte aligned pointers, so a
   routine that hands the caller's result pointer to one must detect an unaligned
   result (`unal8`) and compute into aligned scratch instead. Each call must write
   the same bytes as a call with an aligned result, and touch nothing around them.
   Functions whose result is only a bool or a return value are left out, and so
   is zkvm_keccak_f1600: it is the bare keccakf precompile on the caller's state,
   emitted at the call site with no library routine to test the pointer.
   Output: per case, one byte with bit `o` set when offset `o` is right (0xff when
   all eight are). check.sh compares it with a golden snapshot.
   Build with -DONLY=n to run case n alone (to find the one that halts), and with
   -DTRAP_ON_FAIL to halt on a wrong case (for runners that do not show the
   output, such as the assembly emulator). */
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "zkvm_accelerators.h"
#include "zkvm_u256.h"
#include "zkvm_u256_le.h"
#include "zkvm_zisklib.h"

static volatile uint8_t* const O = (volatile uint8_t*)0xA0410000ULL;

/* little-endian limbs */
static const uint64_t A[4] = {0x0123456789abcdef, 0xfedcba9876543210, 0x0f1e2d3c4b5a6978, 0x1122334455667788};
static const uint64_t B[4] = {0x8877665544332211, 0x0011223344556677, 0x99aabbccddeeff00, 0x0000000000000abc};
static const uint64_t M[4] = {0xffffffff00000001, 0x0000000000000000, 0x00000000ffffffff, 0xffffffff00000000};
static const uint64_t E[4] = {0x10001, 0, 0, 0};
static const uint64_t SH[4] = {77, 0, 0, 0};    /* shift / byte index / signextend size */
static const uint64_t NEG[4] = {1, 0, 0, 0x8000000000000000};
static const uint64_t U_FP[6] = {1, 0, 0, 0, 0, 0};
static const uint64_t U_FP2[12] = {0x0123456789abcdef, 0xfedcba9876543210, 0x0f1e2d3c4b5a6978,
                                   0x1122334455667788, 0, 0, 2, 0, 0, 0, 0, 0};
/* secp256k1 ECDSA vector (as in zisklib_calls.c) */
static const uint64_t R[4] = {0x68df7d8d7e0fb36b, 0xc2189fe681cd6e78, 0xc85ba1fd6238ecb5, 0x3e125456c8338994};
static const uint64_t S[4] = {0xd4e89d1ae75aeea2, 0xb8e33178783bd1a3, 0x0866acebc9e141ec, 0x3a816b1c33739e41};
static const uint64_t Z[4] = {0x1bf86a1816a52f52, 0xd31e26c3da73dda8, 0xa3b71997594da038, 0x17560495f6944673};
static const uint8_t MSG[3] = {'a', 'b', 'c'};
static const uint8_t DST[] = "QUUX-V01-CS02-with-BLS12381G2_XMD:SHA-256_SSWU_RO_";

/* big-endian / byte inputs of the EF functions */
static uint8_t HB[300];
static zkvm_u256 BA, BB, BM, BSH, BNEG;
static zkvm_bn254_g1_point BN_G;                /* (1, 2) */
static zkvm_bn254_scalar BN_K;
static zkvm_secp256k1_hash RMSG;
static zkvm_secp256k1_signature RSIG;
static zkvm_bls12_381_fp BL_U1;
static zkvm_bls12_381_fp2 BL_U2;
static zkvm_bls12_381_g1_point BL_P;            /* map_fp_to_g1(BL_U1) */
static zkvm_bls12_381_g2_point BL_Q;            /* map_fp2_to_g2(BL_U2) */
static zkvm_bls12_381_g1_msm_pair BL_P1;
static zkvm_bls12_381_g2_msm_pair BL_Q1;
static zkvm_blake2f_message BM2;
static zkvm_blake2f_offset BT;

static void be32(uint8_t* d, const uint64_t* le) {
    for (int i = 0; i < 32; i++) d[i] = (uint8_t)(le[3 - i / 8] >> (8 * (7 - i % 8)));
}
static void setup(void) {
    for (int i = 0; i < 300; i++) HB[i] = (uint8_t)(i * 7 + 1);
    be32(BA.data, A); be32(BB.data, B); be32(BM.data, M); be32(BSH.data, SH); be32(BNEG.data, NEG);
    BN_G.data[31] = 1; BN_G.data[63] = 2;
    be32(BN_K.data, A);
    /* ecrecover input: r, s, z of the vector above (big-endian) */
    be32(RSIG.data, R); be32(RSIG.data + 32, S); be32(RMSG.data, Z);
    BL_U1.data[47] = 5;
    BL_U2.data[47] = 3; BL_U2.data[95] = 4;
    zkvm_bls12_map_fp_to_g1(&BL_U1, &BL_P);
    zkvm_bls12_map_fp2_to_g2(&BL_U2, &BL_Q);
    BL_P1.point = BL_P; be32(BL_P1.scalar.data, B);
    BL_Q1.point = BL_Q; be32(BL_Q1.scalar.data, B);
    for (int i = 0; i < 128; i++) BM2.data[i] = (uint8_t)i;
    BT.data[0] = 128;
}

/* Each case writes its result(s) to `out` (in/out results are seeded first). */
typedef void (*case_fn)(uint8_t* out);
#define LE(p) ((zkvm_u256_le*)(p))
#define CLE(p) ((const zkvm_u256_le*)(p))
#define U(p) ((uint64_t*)(p))

/* zkvm_zisklib.h */
static void c_inv256(uint8_t* o) { zkvm_zisklib_inv256(A, U(o)); }
static void c_oadd256(uint8_t* o) { zkvm_zisklib_overflowing_add256(A, B, U(o)); }
static void c_osub256(uint8_t* o) { zkvm_zisklib_overflowing_sub256(B, A, U(o)); }
static void c_omul256(uint8_t* o) { zkvm_zisklib_overflowing_mul256(A, B, U(o)); }
static void c_divrem256(uint8_t* o) { zkvm_zisklib_div_rem256(A, B, U(o), U(o + 32)); }
static void c_reduce_mod256(uint8_t* o) { zkvm_zisklib_reduce_mod256(A, M, U(o)); }
static void c_add_mod256(uint8_t* o) { zkvm_zisklib_add_mod256(A, B, M, U(o)); }
static void c_mul_mod256(uint8_t* o) { zkvm_zisklib_mul_mod256(A, B, M, U(o)); }
static void c_inv_mod256(uint8_t* o) { zkvm_zisklib_inv_mod256(B, M, U(o)); }
static void c_pow_mod256(uint8_t* o) { zkvm_zisklib_pow_mod256(A, E, M, U(o)); }
static void c_opow256(uint8_t* o) { zkvm_zisklib_overflowing_pow256(B, E, U(o)); }
static void c_k1_recover(uint8_t* o) { zkvm_zisklib_ecdsa_recover_secp256k1(R, S, Z, 1, U(o)); }
static void c_map_g1(uint8_t* o) { zkvm_zisklib_map_to_curve_g1_bls12_381(U_FP, U(o)); }
static void c_map_g2(uint8_t* o) { zkvm_zisklib_map_to_curve_g2_bls12_381(U_FP2, U(o)); }
static void c_h2c_g2(uint8_t* o) {
    zkvm_zisklib_hash_to_curve_g2_bls12_381(MSG, 3, DST, sizeof DST - 1, U(o));
}
static void c_modexp_u64(uint8_t* o) { zkvm_zisklib_modexp_u64_c(A, 4, E, 1, M, 4, U(o)); }
static const uint64_t M8[8] = {0x0123456789abcdef, 0xfedcba9876543210, 0x0f1e2d3c4b5a6978, 0x1122334455667788,
                               0x8877665544332211, 0x0011223344556677, 0x99aabbccddeeff00, 0x0000000000000abd};
static void c_modexp_u64_long(uint8_t* o) { zkvm_zisklib_modexp_u64_c(A, 4, E, 1, M8, 8, U(o)); }

/* zkvm_u256_le.h */
#define LE_BIN(n) static void c_le_##n(uint8_t* o) { zkvm_u256_le_##n(CLE(A), CLE(B), LE(o)); }
#define LE_SH(n) static void c_le_##n(uint8_t* o) { zkvm_u256_le_##n(CLE(SH), CLE(A), LE(o)); }
LE_BIN(add) LE_BIN(sub) LE_BIN(mul) LE_BIN(div) LE_BIN(mod) LE_BIN(sdiv) LE_BIN(smod)
LE_BIN(lt) LE_BIN(gt) LE_BIN(slt) LE_BIN(sgt) LE_BIN(eq) LE_BIN(and) LE_BIN(or) LE_BIN(xor)
LE_SH(shl) LE_SH(shr) LE_SH(byte) LE_SH(signextend)
static void c_le_sar(uint8_t* o) { zkvm_u256_le_sar(CLE(SH), CLE(NEG), LE(o)); }
static void c_le_divmod(uint8_t* o) { zkvm_u256_le_divmod(CLE(A), CLE(B), LE(o), LE(o + 32)); }
static void c_le_sdivmod(uint8_t* o) { zkvm_u256_le_sdivmod(CLE(NEG), CLE(B), LE(o), LE(o + 32)); }
/* a negative operand, so the signed results are negated into the output */
static void c_le_sdiv_neg(uint8_t* o) { zkvm_u256_le_sdiv(CLE(NEG), CLE(B), LE(o)); }
static void c_le_smod_neg(uint8_t* o) { zkvm_u256_le_smod(CLE(NEG), CLE(B), LE(o)); }
static void c_le_sdivmod_negb(uint8_t* o) { zkvm_u256_le_sdivmod(CLE(A), CLE(NEG), LE(o), LE(o + 32)); }
static void c_le_addmod(uint8_t* o) { zkvm_u256_le_addmod(CLE(A), CLE(B), CLE(M), LE(o)); }
static void c_le_mulmod(uint8_t* o) { zkvm_u256_le_mulmod(CLE(A), CLE(B), CLE(M), LE(o)); }
static void c_le_exp(uint8_t* o) { zkvm_u256_le_exp(CLE(B), CLE(E), LE(o)); }
static void c_le_iszero(uint8_t* o) { zkvm_u256_le_iszero(CLE(A), LE(o)); }
static void c_le_not(uint8_t* o) { zkvm_u256_le_not(CLE(A), LE(o)); }

/* zkvm_u256.h (EF, big-endian) */
#define BE_BIN(n) static void c_be_##n(uint8_t* o) { zkvm_u256_##n(&BA, &BB, (zkvm_u256*)o); }
#define BE_SH(n) static void c_be_##n(uint8_t* o) { zkvm_u256_##n(&BSH, &BA, (zkvm_u256*)o); }
BE_BIN(add) BE_BIN(sub) BE_BIN(mul) BE_BIN(div) BE_BIN(mod) BE_BIN(sdiv) BE_BIN(smod)
BE_BIN(lt) BE_BIN(gt) BE_BIN(slt) BE_BIN(sgt) BE_BIN(eq) BE_BIN(and) BE_BIN(or) BE_BIN(xor)
BE_SH(shl) BE_SH(shr) BE_SH(byte) BE_SH(signextend)
static void c_be_sar(uint8_t* o) { zkvm_u256_sar(&BSH, &BNEG, (zkvm_u256*)o); }
static void c_be_divmod(uint8_t* o) { zkvm_u256_divmod(&BA, &BB, (zkvm_u256*)o, (zkvm_u256*)(o + 32)); }
static void c_be_sdivmod(uint8_t* o) { zkvm_u256_sdivmod(&BNEG, &BB, (zkvm_u256*)o, (zkvm_u256*)(o + 32)); }
static void c_be_sdiv_neg(uint8_t* o) { zkvm_u256_sdiv(&BNEG, &BB, (zkvm_u256*)o); }
static void c_be_smod_neg(uint8_t* o) { zkvm_u256_smod(&BNEG, &BB, (zkvm_u256*)o); }
static void c_be_addmod(uint8_t* o) { zkvm_u256_addmod(&BA, &BB, &BM, (zkvm_u256*)o); }
static void c_be_mulmod(uint8_t* o) { zkvm_u256_mulmod(&BA, &BB, &BM, (zkvm_u256*)o); }
static void c_be_exp(uint8_t* o) { zkvm_u256_exp(&BB, &BA, (zkvm_u256*)o); }
static void c_be_iszero(uint8_t* o) { zkvm_u256_iszero(&BA, (zkvm_u256*)o); }
static void c_be_not(uint8_t* o) { zkvm_u256_not(&BA, (zkvm_u256*)o); }

/* zkvm_accelerators.h */
static void c_keccak(uint8_t* o) { zkvm_keccak256(HB, 300, (zkvm_keccak256_hash*)o); }
static void c_sha256(uint8_t* o) { zkvm_sha256(HB, 300, (zkvm_sha256_hash*)o); }
static void c_ripemd160(uint8_t* o) { zkvm_ripemd160(HB, 300, (zkvm_ripemd160_hash*)o); }
static void c_ecrecover(uint8_t* o) { zkvm_secp256k1_ecrecover(&RMSG, &RSIG, 1, (zkvm_secp256k1_pubkey*)o); }
static void c_modexp(uint8_t* o) { zkvm_modexp(HB, 37, HB + 40, 19, HB + 64, 48, o); }
static void c_bn_add(uint8_t* o) { zkvm_bn254_g1_add(&BN_G, &BN_G, (zkvm_bn254_g1_point*)o); }
static void c_bn_mul(uint8_t* o) { zkvm_bn254_g1_mul(&BN_G, &BN_K, (zkvm_bn254_g1_point*)o); }
static void c_blake2f(uint8_t* o) {
    for (int i = 0; i < 64; i++) o[i] = HB[i];
    zkvm_blake2f(12, (zkvm_blake2f_state*)o, &BM2, &BT, 1);
}
static void c_bl_g1add(uint8_t* o) { zkvm_bls12_g1_add(&BL_P, &BL_P, (zkvm_bls12_381_g1_point*)o); }
static void c_bl_g1msm(uint8_t* o) { zkvm_bls12_g1_msm(&BL_P1, 1, (zkvm_bls12_381_g1_point*)o); }
static void c_bl_g2add(uint8_t* o) { zkvm_bls12_g2_add(&BL_Q, &BL_Q, (zkvm_bls12_381_g2_point*)o); }
static void c_bl_g2msm(uint8_t* o) { zkvm_bls12_g2_msm(&BL_Q1, 1, (zkvm_bls12_381_g2_point*)o); }
static void c_bl_map1(uint8_t* o) { zkvm_bls12_map_fp_to_g1(&BL_U1, (zkvm_bls12_381_g1_point*)o); }
static void c_bl_map2(uint8_t* o) { zkvm_bls12_map_fp2_to_g2(&BL_U2, (zkvm_bls12_381_g2_point*)o); }

#define C(f, n) {f, n}
static const struct {
    case_fn f;
    size_t len;
} CASES[] = {
    /* 0 */ C(c_inv256, 32), C(c_oadd256, 32), C(c_osub256, 32), C(c_omul256, 32),
    /* 4 */ C(c_divrem256, 64), C(c_reduce_mod256, 32), C(c_add_mod256, 32), C(c_mul_mod256, 32),
    /* 8 */ C(c_inv_mod256, 32), C(c_pow_mod256, 32), C(c_opow256, 32), C(c_k1_recover, 64),
    /* 12 */ C(c_map_g1, 96), C(c_map_g2, 192), C(c_h2c_g2, 192), C(c_modexp_u64, 32),
    /* 16 */ C(c_le_add, 32), C(c_le_sub, 32), C(c_le_mul, 32), C(c_le_div, 32),
    /* 20 */ C(c_le_mod, 32), C(c_le_sdiv, 32), C(c_le_smod, 32), C(c_le_lt, 32),
    /* 24 */ C(c_le_gt, 32), C(c_le_slt, 32), C(c_le_sgt, 32), C(c_le_eq, 32),
    /* 28 */ C(c_le_and, 32), C(c_le_or, 32), C(c_le_xor, 32), C(c_le_shl, 32),
    /* 32 */ C(c_le_shr, 32), C(c_le_byte, 32), C(c_le_signextend, 32), C(c_le_sar, 32),
    /* 36 */ C(c_le_divmod, 64), C(c_le_sdivmod, 64), C(c_le_addmod, 32), C(c_le_mulmod, 32),
    /* 40 */ C(c_le_exp, 32), C(c_le_iszero, 32), C(c_le_not, 32),
    /* 43 */ C(c_be_add, 32), C(c_be_sub, 32), C(c_be_mul, 32), C(c_be_div, 32),
    /* 47 */ C(c_be_mod, 32), C(c_be_sdiv, 32), C(c_be_smod, 32), C(c_be_lt, 32),
    /* 51 */ C(c_be_gt, 32), C(c_be_slt, 32), C(c_be_sgt, 32), C(c_be_eq, 32),
    /* 55 */ C(c_be_and, 32), C(c_be_or, 32), C(c_be_xor, 32), C(c_be_shl, 32),
    /* 59 */ C(c_be_shr, 32), C(c_be_byte, 32), C(c_be_signextend, 32), C(c_be_sar, 32),
    /* 63 */ C(c_be_divmod, 64), C(c_be_sdivmod, 64), C(c_be_addmod, 32), C(c_be_mulmod, 32),
    /* 67 */ C(c_be_exp, 32), C(c_be_iszero, 32), C(c_be_not, 32),
    /* 70 */ C(c_keccak, 32), C(c_sha256, 32), C(c_ripemd160, 32),
    /* 73 */ C(c_ecrecover, 64), C(c_modexp, 48), C(c_bn_add, 64), C(c_bn_mul, 64),
    /* 77 */ C(c_blake2f, 64), C(c_bl_g1add, 96), C(c_bl_g1msm, 96), C(c_bl_g2add, 192),
    /* 81 */ C(c_bl_g2msm, 192), C(c_bl_map1, 96), C(c_bl_map2, 192),
    /* 84 */ C(c_le_sdiv_neg, 32), C(c_le_smod_neg, 32), C(c_le_sdivmod_negb, 64), C(c_be_sdiv_neg, 32),
    /* 88 */ C(c_be_smod_neg, 32), C(c_modexp_u64_long, 64),
};
#define NCASES (sizeof(CASES) / sizeof(CASES[0]))

#define MAX_LEN 192
#define GUARD 8
#define FILL 0xA5

static uint64_t ref[MAX_LEN / 8];
static uint64_t buf[(MAX_LEN + 2 * GUARD) / 8];

/* Bit o of the result: offset o wrote the aligned result's bytes and nothing else. */
static uint8_t check(size_t c) {
    size_t len = CASES[c].len;
    for (size_t i = 0; i < sizeof(ref); i++) ((uint8_t*)ref)[i] = 0;
    CASES[c].f((uint8_t*)ref);
    const uint8_t* e = (const uint8_t*)ref;
    uint8_t mask = 0;
    for (int o = 0; o < 8; o++) {
        uint8_t* b = (uint8_t*)buf;
        for (size_t i = 0; i < sizeof(buf); i++) b[i] = FILL;
        CASES[c].f(b + GUARD + o);
        int ok = 1;
        size_t lo = GUARD + o;
        for (size_t i = 0; i < sizeof(buf); i++) {
            uint8_t want = (i >= lo && i < lo + len) ? e[i - lo] : FILL;
            if (b[i] != want) ok = 0;
        }
        mask |= (uint8_t)(ok << o);
    }
    return mask;
}

int main(void) {
    setup();
#ifdef ONLY
    O[0] = check(ONLY);
#else
    for (size_t c = 0; c < NCASES; c++) {
        O[c] = check(c);
#ifdef TRAP_ON_FAIL
        if (O[c] != 0xff) __builtin_trap();
#endif
    }
#endif
    return 0;
}
