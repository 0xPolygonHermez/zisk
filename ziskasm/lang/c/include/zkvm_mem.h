/*
 * zkvm_mem.h — memory operations for ZisK guests.
 *
 * NOT part of the EF standard: a ZisK extension, like zkvm_u256_le.h. On ZisK each
 * function is one DMA precompile, defined inline here (as the EF header does for
 * zkvm_keccak_f1600): a `csrs` marker plus the `add`/`addi` that follows, which the
 * transpiler folds into a single DMA operation at the call site. No call is made.
 * A size that is a compile-time constant (up to 2047) travels as the `addi`
 * immediate, which makes the copy, compare or fill ONE ZisK instruction; a size in
 * a register costs one more, which writes the count for the DMA op.
 *
 * The results are libc's. zkvm_memcpy copies correctly between overlapping
 * regions, so it is also a memmove. zkvm_memset needs its fill byte as an
 * immediate: with a run-time fill it calls zkvm_memset_any (src/zkvm_mem.s), which
 * picks the immediate from a jump table. src/zkvm_mem.s also defines weak libc
 * memcpy / memmove / memcmp / memset on the same DMA ops, for freestanding guests
 * (the compiler emits calls to those names).
 *
 * Other targets (host builds) get the declarations only.
 */
#ifndef ZKVM_MEM_H
#define ZKVM_MEM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Sets n bytes at dst to (unsigned char)c through a jump table; returns dst. The
 * out-of-line path of zkvm_memset for a fill byte known only at run time. */
void* zkvm_memset_any(void* dst, int c, size_t n);

#if defined(__riscv)

#define ZKVM_MEM_I static inline __attribute__((always_inline))

/* The immediate forms need a size the optimizer has reduced to a constant. */
#if defined(__OPTIMIZE__)
#define ZKVM_MEM_IMM(n) (__builtin_constant_p(n) && (n) <= 2047)
#else
#define ZKVM_MEM_IMM(n) 0
#endif

/* Copies n bytes from src to dst, which may overlap; returns dst. */
ZKVM_MEM_I void* zkvm_memcpy(void* dst, const void* src, size_t n) {
    if (ZKVM_MEM_IMM(n))
        __asm__ volatile("csrs 0x813, %1\n\taddi x0, %0, %2"
                         : : "r"(dst), "r"(src), "i"(n) : "memory");
    else
        __asm__ volatile("csrs 0x813, %1\n\tadd x0, %0, %2"
                         : : "r"(dst), "r"(src), "r"(n) : "memory");
    return dst;
}

/* Compares n bytes of a and b as unsigned chars: the difference of the first pair
 * that differs (< 0 or > 0), 0 if all n are equal. */
ZKVM_MEM_I int zkvm_memcmp(const void* a, const void* b, size_t n) {
    int64_t r;
    if (ZKVM_MEM_IMM(n))
        __asm__ volatile("csrrs %0, 0x814, %2\n\taddi x0, %1, %3"
                         : "=r"(r) : "r"(a), "r"(b), "i"(n) : "memory");
    else
        __asm__ volatile("csrrs %0, 0x814, %2\n\tadd x0, %1, %3"
                         : "=r"(r) : "r"(a), "r"(b), "r"(n) : "memory");
    return (int)r;
}

/* Sets n bytes at dst to (unsigned char)c; returns dst. */
ZKVM_MEM_I void* zkvm_memset(void* dst, int c, size_t n) {
    if (!ZKVM_MEM_IMM(c)) return zkvm_memset_any(dst, c, n);
    if (ZKVM_MEM_IMM(n))
        __asm__ volatile("csrsi 0x816, 2\n\taddi x0, %0, %1\n\taddi x0, %0, %2"
                         : : "r"(dst), "i"(n), "i"(c & 0xff) : "memory");
    else
        __asm__ volatile("csrs 0x816, %0\n\taddi x0, %1, %2"
                         : : "r"(dst), "r"(n), "i"(c & 0xff) : "memory");
    return dst;
}

#undef ZKVM_MEM_IMM
#undef ZKVM_MEM_I

#else /* !__riscv */

void* zkvm_memcpy(void* dst, const void* src, size_t n);
int zkvm_memcmp(const void* a, const void* b, size_t n);
void* zkvm_memset(void* dst, int c, size_t n);

#endif

#ifdef __cplusplus
}
#endif

#endif /* ZKVM_MEM_H */
