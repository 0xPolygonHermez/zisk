/*
 * zkvm_random.h — Ethereum Foundation zkVM host randomness C interface. Mirrors
 * the standard at:
 *   github.com/eth-act/zkevm-standards (standards/host-randomness)
 *
 * ZisK implements it with no C layer, like the accelerators: `zkvm_random_u64` is
 * a zkvmcall thunk in src/zkvm_calls.s (`csrs <id>, x0; ret`), which the transpiler
 * turns into a jump to `ziskasm_zkvm_random_u64` in
 * ziskasm/zisklib/zkvm/random.zisk. The host provides a 256-bit seed per execution,
 * and the values are the SHAKE256 output stream of that seed. A guest compiles
 * against this header and links `zisklib_c` (see CMakeLists.txt).
 */
#ifndef ZKVM_RANDOM_H
#define ZKVM_RANDOM_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Return 64 uniformly distributed random bits.
 *
 * Each call returns bits independent of every other call. The host draws them
 * from a cryptographically secure pseudorandom number generator, independently
 * for each guest execution, so the author of the guest input cannot predict
 * them. A guest that needs more bits calls again rather than expanding one
 * value itself.
 *
 * The function cannot fail, so no error code is returned.
 *
 * The values are free witness values and carry no attestation. For a fixed
 * guest input, the public output and the exit code must be identical for every
 * sequence this function can return.
 *
 * @return 64 random bits
 */
uint64_t zkvm_random_u64(void);

#ifdef __cplusplus
}
#endif

#endif /* ZKVM_RANDOM_H */
