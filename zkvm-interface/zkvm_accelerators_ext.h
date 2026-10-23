/**
 * zkVM Non-Standard Accelerator Extensions (ZisK)
 *
 * These are ZisK-specific accelerators that are NOT part of the standard EVM-precompile
 * C interface in zkvm_accelerators.h. They are kept separate so the standard interface stays
 * limited to standardized operations; any of these may be promoted into the standard header
 * if/when standardized.
 *
 * Unless stated otherwise, operands are 32-byte big-endian; the Ed25519 accelerators use the
 * RFC 8032 (little-endian) encodings. All functions follow the same conventions as the
 * standard accelerators (caller allocates input/output; NULL pointers SHOULD panic).
 */

#ifndef ZKVM_ACCELERATORS_EXT_H
#define ZKVM_ACCELERATORS_EXT_H

#include <stdint.h>

/* Pulls in zkvm_status and the ZKVM_EOK / ZKVM_EFAIL status codes. */
#include "zkvm_accelerators.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * 256-bit modular multiplication: output = (a * b) mod m
 *
 * Used by the EVM MULMOD opcode (0x09). All operands are 32-byte big-endian.
 *
 * @param a Pointer to first operand (32 bytes, big-endian)
 * @param b Pointer to second operand (32 bytes, big-endian)
 * @param m Pointer to modulus (32 bytes, big-endian)
 * @param[out] output Pointer to output buffer (32 bytes, big-endian)
 * @return ZKVM_EOK on success, ZKVM_EFAIL on failure
 */
zkvm_status zkvm_mulmod256(const uint8_t* a, const uint8_t* b,
                           const uint8_t* m, uint8_t* output);

/**
 * 256-bit modular reduction: output = a mod m
 *
 * All operands are 32-byte big-endian.
 *
 * @param a Pointer to operand (32 bytes, big-endian)
 * @param m Pointer to modulus (32 bytes, big-endian)
 * @param[out] output Pointer to output buffer (32 bytes, big-endian)
 * @return ZKVM_EOK on success, ZKVM_EFAIL on failure
 */
zkvm_status zkvm_reduce_mod256(const uint8_t* a, const uint8_t* m, uint8_t* output);

/**
 * 256-bit modular addition: output = (a + b) mod m
 *
 * All operands are 32-byte big-endian.
 *
 * @param a Pointer to first operand (32 bytes, big-endian)
 * @param b Pointer to second operand (32 bytes, big-endian)
 * @param m Pointer to modulus (32 bytes, big-endian)
 * @param[out] output Pointer to output buffer (32 bytes, big-endian)
 * @return ZKVM_EOK on success, ZKVM_EFAIL on failure
 */
zkvm_status zkvm_add_mod256(const uint8_t* a, const uint8_t* b,
                            const uint8_t* m, uint8_t* output);

/**
 * 256-bit modular squaring: output = a^2 mod m
 *
 * All operands are 32-byte big-endian.
 *
 * @param a Pointer to operand (32 bytes, big-endian)
 * @param m Pointer to modulus (32 bytes, big-endian)
 * @param[out] output Pointer to output buffer (32 bytes, big-endian)
 * @return ZKVM_EOK on success, ZKVM_EFAIL on failure
 */
zkvm_status zkvm_square_mod256(const uint8_t* a, const uint8_t* m, uint8_t* output);

/**
 * 256-bit modular exponentiation: output = base^exp mod m
 *
 * All operands are 32-byte big-endian.
 *
 * @param base Pointer to base (32 bytes, big-endian)
 * @param exp Pointer to exponent (32 bytes, big-endian)
 * @param m Pointer to modulus (32 bytes, big-endian)
 * @param[out] output Pointer to output buffer (32 bytes, big-endian)
 * @return ZKVM_EOK on success, ZKVM_EFAIL on failure
 */
zkvm_status zkvm_pow_mod256(const uint8_t* base, const uint8_t* exp,
                            const uint8_t* m, uint8_t* output);

/**
 * 256-bit modular inverse: output = a^(-1) mod m
 *
 * All operands are 32-byte big-endian.
 *
 * @param a Pointer to operand (32 bytes, big-endian)
 * @param m Pointer to modulus (32 bytes, big-endian)
 * @param[out] output Pointer to output buffer (32 bytes, big-endian)
 * @return ZKVM_EOK if the inverse exists, ZKVM_EFAIL otherwise
 */
zkvm_status zkvm_inv_mod256(const uint8_t* a, const uint8_t* m, uint8_t* output);

/**
 * Ed25519 signature verification (RFC 8032, pure Ed25519), with ed25519-dalek `verify`
 * semantics: cofactorless, S >= L rejected, A decompressed as curve25519-dalek does.
 *
 * @param pk Pointer to the compressed public key A (32 bytes)
 * @param msg Pointer to the message (msg_len bytes; may be NULL if msg_len is 0)
 * @param msg_len Message length in bytes
 * @param sig Pointer to the signature R || S (64 bytes)
 * @param[out] verified true if the signature is valid
 * @return ZKVM_EOK on success
 */
zkvm_status zkvm_ed25519_verify(const uint8_t* pk, const uint8_t* msg, size_t msg_len,
                                const uint8_t* sig, bool* verified);

/**
 * Ed25519 signature verification with ed25519-dalek `verify_strict` semantics: as
 * zkvm_ed25519_verify, and additionally R must decompress and neither A nor R may have
 * small order.
 *
 * @param pk Pointer to the compressed public key A (32 bytes)
 * @param msg Pointer to the message (msg_len bytes; may be NULL if msg_len is 0)
 * @param msg_len Message length in bytes
 * @param sig Pointer to the signature R || S (64 bytes)
 * @param[out] verified true if the signature is valid
 * @return ZKVM_EOK on success
 */
zkvm_status zkvm_ed25519_verify_strict(const uint8_t* pk, const uint8_t* msg, size_t msg_len,
                                       const uint8_t* sig, bool* verified);

/**
 * Ed25519 point decompression with curve25519-dalek `CompressedEdwardsY::decompress`
 * semantics (y taken mod p, a sign bit on x = 0 ignored).
 *
 * @param compressed Pointer to the compressed point (32 bytes, RFC 8032 encoding)
 * @param[out] point Pointer to output buffer (64 bytes): x || y, each 32-byte little-endian
 *             and canonical
 * @return ZKVM_EOK if `compressed` encodes a curve point, ZKVM_EFAIL otherwise
 */
zkvm_status zkvm_ed25519_decompress(const uint8_t* compressed, uint8_t* point);

/**
 * Ed25519 double scalar multiplication with the base point: result = [k]A + [s]B, with B the
 * Ed25519 base point. Variable time, as curve25519-dalek
 * `EdwardsPoint::vartime_double_scalar_mul_basepoint`.
 *
 * @param k Pointer to the scalar of A (32 bytes, little-endian; any 256-bit value)
 * @param point Pointer to the point A (64 bytes): x || y, each 32-byte little-endian
 * @param s Pointer to the scalar of B (32 bytes, little-endian; any 256-bit value)
 * @param[out] result Pointer to output buffer (64 bytes): x || y, each 32-byte little-endian
 *             and canonical
 * @return ZKVM_EOK on success, ZKVM_EFAIL if A is not a curve point with canonical coordinates
 */
zkvm_status zkvm_ed25519_double_scalar_mul_base(const uint8_t* k, const uint8_t* point,
                                                const uint8_t* s, uint8_t* result);

#ifdef __cplusplus
}
#endif

#endif /* ZKVM_ACCELERATORS_EXT_H */
