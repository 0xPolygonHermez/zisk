/*
 * zkvm_evm.h — EVM helpers for ZisK guests.
 *
 * NOT part of the EF standard: a ZisK extension, like zkvm_mem.h. On ZisK each
 * function is one EVM precompile, defined inline here: a `csrs` marker plus the
 * `add` that follows, which the transpiler folds into the precompile at the call
 * site. Other targets (host builds) get the declarations only.
 */
#ifndef ZKVM_EVM_H
#define ZKVM_EVM_H

#include <stddef.h>
#include <stdint.h>

#include "zkvm_accelerators.h"  /* zkvm_status */

#ifdef __cplusplus
extern "C" {
#endif

/* JUMPDEST analysis of EVM bytecode: writes ceil(size/64) little-endian words to
 * bitmap, bit i (word i/64, bit i%64) set iff code[i] is a JUMPDEST (0x5b) that is
 * not PUSH immediate data. Every word is written, including zero words, and the
 * last one in full even when the code ends part way into it.
 *
 * The precompile requires code and bitmap 8-byte aligned and size > 0. When they
 * are not, nothing is written and ZKVM_EFAIL is returned: the caller analyses the
 * code itself (bytecode inside an input buffer is often unaligned). ZKVM_EOK
 * otherwise. */
#if defined(__riscv)
static inline __attribute__((always_inline)) zkvm_status zkvm_evm_jumpdest_bitmap(
    const uint8_t* code, size_t size, uint64_t* bitmap) {
    if (size == 0 || (((uintptr_t)code | (uintptr_t)bitmap) & 7) != 0) return ZKVM_EFAIL;
    __asm__ volatile("csrs 0x81c, %1\n\tadd x0, %0, %2"
                     : : "r"(bitmap), "r"(code), "r"(size) : "memory");
    return ZKVM_EOK;
}
#else
zkvm_status zkvm_evm_jumpdest_bitmap(const uint8_t* code, size_t size, uint64_t* bitmap);
#endif

#ifdef __cplusplus
}
#endif

#endif /* ZKVM_EVM_H */
