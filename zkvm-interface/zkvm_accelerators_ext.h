/**
 * zkVM Non-Standard Accelerator Extensions (ZisK)
 *
 * These are ZisK-specific accelerators that are NOT part of the standard EVM-precompile
 * C interface in zkvm_accelerators.h. They are kept separate so the standard interface stays
 * limited to standardized operations; any of these may be promoted into the standard header
 * if/when standardized.
 *
 * All operands are 32-byte big-endian, and all functions follow the same conventions as the
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

#ifdef __cplusplus
}
#endif

#endif /* ZKVM_ACCELERATORS_EXT_H */
