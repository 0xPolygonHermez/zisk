/*
 * zkvm_fcall.h — free-input calls (fcalls) for ZisK guests: unverified hints.
 *
 * NOT part of the EF standard: a ZisK extension, like zkvm_zisklib.h. An fcall
 * asks the executor for values that are expensive to compute and cheap to check,
 * such as a field inverse or a square root. The proof does not cover how they were
 * computed: the guest must check every result (for an inverse, that x * inv == 1)
 * before relying on it, or a dishonest prover can feed it any value.
 *
 * zkvm_fcall passes the parameter words and issues the hint; zkvm_fcall_get then
 * returns the result words one at a time, in order (zkvm_fcall_read reads several
 * into a buffer). Every hint below has a fixed layout: read exactly as many result
 * words as it produces, or, for the variable-length ones (BIN_DECOMP, BIGINT_DIV),
 * read each length first. Reading past the last result word halts the executor.
 * Wrong parameters (for example too few, or a zero to invert or to take the most
 * significant bit of) may halt it too: either way no proof of the run exists.
 *
 * On RISC-V the three functions are in src/zkvm_fcall.s, which zisklib_c links;
 * compiled for wasm they are imports, which wasm2rom implements the same way.
 * Field elements are little-endian uint64_t limbs, and an Fp2 element is its real
 * part followed by its imaginary part.
 */
#ifndef ZKVM_FCALL_H
#define ZKVM_FCALL_H

#include <stddef.h>
#include <stdint.h>

#include "zkvm_accelerators.h"  /* zkvm_status */

#ifdef __cplusplus
extern "C" {
#endif

/* The hints: parameter words -> result words. "is_qr" is 1 when the input is a
 * square, and then the root follows; when it is 0, the root of x * NQR follows
 * instead (NQR being the field's fixed non-residue), which proves it is not. */
#define ZKVM_FCALL_SECP256K1_FP_INV                 1  /* x[4] -> inv[4] */
#define ZKVM_FCALL_SECP256K1_FN_INV                 2  /* x[4] -> inv[4] */
#define ZKVM_FCALL_SECP256K1_FP_SQRT                3  /* x[4], parity -> is_qr, root[4] */
#define ZKVM_FCALL_SECP256K1_GLV_DECOMPOSE          4  /* k[4] -> |k1|[4], |k2|[4], neg1, neg2 */
#define ZKVM_FCALL_SECP256R1_FN_INV                 5  /* x[4] -> inv[4] */
#define ZKVM_FCALL_BN254_FP_INV                     6  /* x[4] -> inv[4] */
#define ZKVM_FCALL_BN254_FP2_INV                    7  /* x[8] -> inv[8] */
#define ZKVM_FCALL_BN254_TWIST_ADD_LINE_COEFFS      8  /* x1, y1, x2, y2 [8 each] -> lambda[8], mu[8] */
#define ZKVM_FCALL_BN254_TWIST_DBL_LINE_COEFFS      9  /* x[8], y[8] -> lambda[8], mu[8] */
#define ZKVM_FCALL_BLS12_381_FP_INV                10  /* x[6] -> inv[6] */
#define ZKVM_FCALL_BLS12_381_FP_SQRT               11  /* x[6] -> is_qr, root[6] */
#define ZKVM_FCALL_BLS12_381_FP2_INV               12  /* x[12] -> inv[12] */
#define ZKVM_FCALL_BLS12_381_FP2_SQRT              13  /* x[12] -> is_qr, root[12] */
#define ZKVM_FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS 14  /* x1, y1, x2, y2 [12 each] -> lambda[12], mu[12] */
#define ZKVM_FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS 15  /* x[12], y[12] -> lambda[12], mu[12] */
#define ZKVM_FCALL_BIN_DECOMP                      16  /* n, x[n] -> m, bits[m] (most significant first) */
#define ZKVM_FCALL_MSB_POS_256                     17  /* n, x_1[4] .. x_n[4] -> limb, bit */
#define ZKVM_FCALL_MSB_POS_384                     18  /* x[6], 0, 0, y[6] -> limb, bit */
#define ZKVM_FCALL_UINT256_DIV                     19  /* a[4], b[4] -> q[4], r[4] */
#define ZKVM_FCALL_UINT256_INV                     20  /* a[4] -> ok, inv[4] (mod 2^256) */
#define ZKVM_FCALL_UINT256_INV_MOD                 21  /* a[4], m[4] -> ok, inv[4], 0[8], or
                                                          0, gcd[4], a/gcd[4], m/gcd[4] */
#define ZKVM_FCALL_BIGINT_DIV                      22  /* na, a[na], nb, b[nb] -> nq, q[nq], nr, r[nr],
                                                          each zero-padded to a multiple of 4 limbs */
#define ZKVM_FCALL_ED25519_FP_SQRT_RATIO           26  /* u[4], v[4] -> is_square, root[4]: the even
                                                          root of u/v, or of 2u/v when it is not a square */

/* The most parameter words an fcall takes. */
#define ZKVM_FCALL_MAX_PARAMS 386

/* Passes params[0..n_params] and issues hint `id`; its results are then read with
 * zkvm_fcall_get / zkvm_fcall_read. ZKVM_EFAIL, with nothing passed or issued, when
 * `id` is not one of the hints above or n_params > ZKVM_FCALL_MAX_PARAMS. */
zkvm_status zkvm_fcall(uint32_t id, const uint64_t* params, size_t n_params);

/* The next result word of the last hint issued. */
uint64_t zkvm_fcall_get(void);

/* Reads the next n result words of the last hint issued into results[0..n]. */
void zkvm_fcall_read(uint64_t* results, size_t n);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ZKVM_FCALL_H */
