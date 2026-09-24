/* u256_bench_guest.c -- step cost of the EF U256 ABI (zkvm_u256.h) under its two
 * ZisK implementations. The guest code is identical; only the implementation
 * behind the standard zkvm_u256_* functions changes:
 *   - default: zkvmcall thunks, i.e. a call that the transpiler turns into a jump
 *     to the .zisk routine;
 *   - -DZKVM_U256_INLINE: the static inline C versions from zkvm_u256_inline.h
 *     (the division family has none and stays a call).
 * -DU256_LE runs the same loop through the little-endian variant instead
 * (zkvm_u256_le.h), which has the same two implementations: calls by default,
 * -DZKVM_U256_LE_INLINE for inline. The operands are converted once before the
 * loop and the result once after it, so all four builds of an operation emit the
 * same bytes.
 *
 * One operation per build: -DOP=<name> (e.g. -DOP=add) plus its operand shape,
 * one of -DKIND_BIN (op(a, b, r)), -DKIND_SHIFT (op(s, a, r): shl/shr/sar/byte/
 * signextend), -DKIND_UN (op(a, r)), -DKIND_TER (op(a, b, m, r)), -DKIND_DIVMOD
 * (op(a, b, q, r)) or -DKIND_NOP (empty loop, for the loop's own cost).
 *
 * Each iteration runs the operation once on fixed operands; a compiler barrier
 * makes the inline versions recompute every time. The result is emitted, so both
 * builds of one operation must produce the same output.
 *
 *   riscv-none-elf-gcc -march=rv64ima_zicsr_zbb -mabi=lp64 -mcmodel=medany \
 *       -nostdlib -ffreestanding -O2 -Wl,--gc-sections -I. -I../include \
 *       -T ../../../../ziskbuild/zisk_linker_script.ld -DOP=add -DKIND_BIN \
 *       [-DZKVM_U256_INLINE | -DU256_LE [-DZKVM_U256_LE_INLINE]] [-DU256_N=1000] \
 *       -o u256.elf ../src/_start.s u256_bench_guest.c ../src/zkvm_calls.s
 *   ziskemu -e u256.elf -i empty.bin -o out.bin -X
 */
#include "zkvm_u256.h"
#include "zkvm_u256_le.h"
#include "emit.h"

#ifndef U256_N
#define U256_N 1000
#endif

/* ---- the benchmark -------------------------------------------------------- */

#define CAT(a, b) a##b
#define XCAT(a, b) CAT(a, b)
#ifdef U256_LE
#define FN XCAT(zkvm_u256_le_, OP)
#else
#define FN XCAT(zkvm_u256_, OP)
#endif

/* Operands: a is a full 256-bit value with the top bit set (negative when signed),
 * b a 128-bit value, m a 255-bit modulus, s a shift/index/byte-position operand.
 * They are global (not static) so the compiler barrier forces the inline versions
 * to reload them every iteration, as the called routines do. */
zkvm_u256 A = {{0x9e, 0x37, 0x79, 0xb9, 0x7f, 0x4a, 0x7c, 0x15, 0xf3, 0x9c, 0xc0, 0x60,
                       0x5c, 0xed, 0xc8, 0x34, 0x10, 0x82, 0x27, 0x6b, 0xf3, 0xa2, 0x72, 0x51,
                       0xf8, 0x6c, 0x6a, 0x11, 0xd0, 0xc1, 0x8e, 0x95}};
zkvm_u256 B = {{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                       0xd1, 0xb5, 0x4a, 0x32, 0xd1, 0x92, 0xed, 0x03,
                       0x94, 0xd8, 0x8c, 0xa5, 0xe1, 0x97, 0x60, 0x2f}};
zkvm_u256 M = {{0x73, 0xed, 0xa7, 0x53, 0x29, 0x9d, 0x7d, 0x48, 0x33, 0x39, 0xd8, 0x08,
                       0x09, 0xa1, 0xd8, 0x05, 0x53, 0xbd, 0xa4, 0x02, 0xff, 0xfe, 0x5b, 0xfe,
                       0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01}};
zkvm_u256 S = {{[31] = 13}};
zkvm_u256 R, Q;

#ifdef U256_LE
zkvm_u256_le LA, LB, LM, LS, LR, LQ;
static void be_to_le(const zkvm_u256* x, zkvm_u256_le* y) {
    for (int i = 0; i < 4; i++) {
        uint64_t v = 0;
        for (int j = 0; j < 8; j++) v = (v << 8) | x->data[8 * (3 - i) + j];
        y->limbs[i] = v;
    }
}
static void le_to_be(const zkvm_u256_le* x, zkvm_u256* y) {
    for (int i = 0; i < 32; i++) y->data[i] = (uint8_t)(x->limbs[(31 - i) / 8] >> (8 * ((31 - i) % 8)));
}
#define OPND(x) (&L##x)
#else
#define OPND(x) (&x)
#endif

int main(void) {
#ifdef U256_LE
    be_to_le(&A, &LA); be_to_le(&B, &LB); be_to_le(&M, &LM); be_to_le(&S, &LS);
#endif
    unsigned failures = 0;
    for (unsigned i = 0; i < U256_N; i++) {
        zkvm_status st = ZKVM_EOK;
#if defined(KIND_BIN)
        st = FN(OPND(A), OPND(B), OPND(R));
#elif defined(KIND_SHIFT)
        st = FN(OPND(S), OPND(A), OPND(R));
#elif defined(KIND_UN)
        st = FN(OPND(A), OPND(R));
#elif defined(KIND_TER)
        st = FN(OPND(A), OPND(B), OPND(M), OPND(R));
#elif defined(KIND_DIVMOD)
        st = FN(OPND(A), OPND(B), OPND(Q), OPND(R));
#elif !defined(KIND_NOP)
#error "pick an operand shape: -DKIND_BIN, _SHIFT, _UN, _TER, _DIVMOD or _NOP"
#endif
        if (st != ZKVM_EOK) failures++;
        __asm__ volatile("" : : : "memory");   /* recompute every iteration */
    }
    if (failures) __asm__ volatile("unimp");
#ifdef U256_LE
    le_to_be(&LR, &R);
#endif
    emit32(R.data);
    return 0;
}
