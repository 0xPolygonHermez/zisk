/* u256_le_guest.c -- conformance of the little-endian U256 ABI (zkvm_u256_le.h),
 * checked differentially against the big-endian EF ABI (zkvm_u256.h), which
 * u256_semantics_guest.c and u256_alias_guest.c already validate.
 *
 * For every operation and every combination of the 16 operand values below
 * (every pair for binary ops, every triple for addmod/mulmod), the LE function's
 * result, converted to big-endian, must equal the BE function's. Each LE call is
 * then repeated with the result aliasing each input in turn, and must give the
 * same result again.
 *
 * Output, self-checking (no golden vector): 27 bytes, one per operation in
 * zkvm_u256.h order, 00 = every case matched, 01 = a mismatch; then 27 bytes for
 * the aliasing checks; then 01 00, a negative control (a deliberate mismatch,
 * then a match) so an all-zero output can't come from a harness that compared
 * nothing. Expected: 54 zero bytes, then 0100.
 *
 *   riscv-none-elf-gcc -march=rv64ima_zicsr_zbb -mabi=lp64 -mcmodel=medany \
 *       -nostdlib -ffreestanding -O2 -Wl,--gc-sections -I. -I../include \
 *       -T ../../../../ziskbuild/zisk_linker_script.ld \
 *       -o u256le.elf ../src/_start.s u256_le_guest.c ../src/zkvm_calls.s
 */
#include "zkvm_u256.h"
#include "zkvm_u256_le.h"

#define NV 16
/* Operand values, as little-endian limbs. */
static const uint64_t VALS[NV][4] = {
    {0, 0, 0, 0},
    {1, 0, 0, 0},
    {7, 0, 0, 0},
    {31, 0, 0, 0},
    {32, 0, 0, 0},
    {64, 0, 0, 0},
    {255, 0, 0, 0},
    {256, 0, 0, 0},
    {~0ull, 0, 0, 0},                                            /* 2^64 - 1 */
    {0, 1, 0, 0},                                                /* 2^64 */
    {0x94d88ca5e197602full, 0xd1b54a32d192ed03ull, 0, 0},        /* 128-bit */
    {0xffffffff00000001ull, 0x53bda402fffe5bfeull, 0x3339d80809a1d805ull,
     0x73eda753299d7d48ull},                                     /* 255-bit */
    {0xf86c6a11d0c18e95ull, 0x1082276bf3a27251ull, 0xf39cc0605cedc834ull,
     0x9e3779b97f4a7c15ull},                                     /* negative */
    {0, 0, 0, 1ull << 63},                                       /* -2^255 */
    {~0ull, ~0ull, ~0ull, ~0ull >> 1},                           /* 2^255 - 1 */
    {~0ull, ~0ull, ~0ull, ~0ull},                                /* -1 */
};

static void to_be(const zkvm_u256_le* x, zkvm_u256* out) {
    for (int i = 0; i < 32; i++) out->data[i] = (uint8_t)(x->limbs[(31 - i) / 8] >> (8 * ((31 - i) % 8)));
}
static int same(const zkvm_u256_le* x, const zkvm_u256* be) {
    zkvm_u256 t;
    to_be(x, &t);
    for (int i = 0; i < 32; i++)
        if (t.data[i] != be->data[i]) return 0;
    return 1;
}
static int same_le(const zkvm_u256_le* x, const zkvm_u256_le* y) {
    for (int i = 0; i < 4; i++)
        if (x->limbs[i] != y->limbs[i]) return 0;
    return 1;
}

static zkvm_u256_le LA, LB, LN, LR, LR2, T;
static zkvm_u256 BA, BB, BN, BR, BR2;
static uint8_t bad[27], bad_alias[27];

static void load(int i, int j, int k) {
    for (int w = 0; w < 4; w++) {
        LA.limbs[w] = VALS[i][w];
        LB.limbs[w] = VALS[j][w];
        LN.limbs[w] = VALS[k][w];
    }
    to_be(&LA, &BA); to_be(&LB, &BB); to_be(&LN, &BN);
}

/* Binary op: compare with BE, then alias the result onto a, then onto b. */
#define CHECK_BIN(idx, op)                                                        \
    for (int i = 0; i < NV; i++)                                                  \
        for (int j = 0; j < NV; j++) {                                            \
            load(i, j, 0);                                                        \
            if (zkvm_u256_##op(&BA, &BB, &BR) != ZKVM_EOK) bad[idx] = 1;          \
            if (zkvm_u256_le_##op(&LA, &LB, &LR) != ZKVM_EOK) bad[idx] = 1;       \
            if (!same(&LR, &BR)) bad[idx] = 1;                                    \
            T = LA; zkvm_u256_le_##op(&T, &LB, &T);                               \
            if (!same_le(&T, &LR)) bad_alias[idx] = 1;                            \
            T = LB; zkvm_u256_le_##op(&LA, &T, &T);                               \
            if (!same_le(&T, &LR)) bad_alias[idx] = 1;                            \
        }

#define CHECK_UN(idx, op)                                                         \
    for (int i = 0; i < NV; i++) {                                                \
        load(i, 0, 0);                                                            \
        if (zkvm_u256_##op(&BA, &BR) != ZKVM_EOK) bad[idx] = 1;                   \
        if (zkvm_u256_le_##op(&LA, &LR) != ZKVM_EOK) bad[idx] = 1;                \
        if (!same(&LR, &BR)) bad[idx] = 1;                                        \
        T = LA; zkvm_u256_le_##op(&T, &T);                                        \
        if (!same_le(&T, &LR)) bad_alias[idx] = 1;                                \
    }

#define CHECK_TER(idx, op)                                                        \
    for (int i = 0; i < NV; i++)                                                  \
        for (int j = 0; j < NV; j++)                                              \
            for (int k = 0; k < NV; k++) {                                        \
                load(i, j, k);                                                    \
                if (zkvm_u256_##op(&BA, &BB, &BN, &BR) != ZKVM_EOK) bad[idx] = 1; \
                if (zkvm_u256_le_##op(&LA, &LB, &LN, &LR) != ZKVM_EOK) bad[idx] = 1; \
                if (!same(&LR, &BR)) bad[idx] = 1;                                \
                T = LA; zkvm_u256_le_##op(&T, &LB, &LN, &T);                      \
                if (!same_le(&T, &LR)) bad_alias[idx] = 1;                        \
                T = LB; zkvm_u256_le_##op(&LA, &T, &LN, &T);                      \
                if (!same_le(&T, &LR)) bad_alias[idx] = 1;                        \
                T = LN; zkvm_u256_le_##op(&LA, &LB, &T, &T);                      \
                if (!same_le(&T, &LR)) bad_alias[idx] = 1;                        \
            }

/* Two outputs (quotient, remainder): each may alias either input. */
#define CHECK_DIVMOD(idx, op)                                                     \
    for (int i = 0; i < NV; i++)                                                  \
        for (int j = 0; j < NV; j++) {                                            \
            load(i, j, 0);                                                        \
            if (zkvm_u256_##op(&BA, &BB, &BR, &BR2) != ZKVM_EOK) bad[idx] = 1;    \
            if (zkvm_u256_le_##op(&LA, &LB, &LR, &LR2) != ZKVM_EOK) bad[idx] = 1; \
            if (!same(&LR, &BR) || !same(&LR2, &BR2)) bad[idx] = 1;               \
            zkvm_u256_le q, r;                                                    \
            q = LA; r = LB; zkvm_u256_le_##op(&q, &r, &q, &r);                    \
            if (!same_le(&q, &LR) || !same_le(&r, &LR2)) bad_alias[idx] = 1;      \
            q = LB; r = LA; zkvm_u256_le_##op(&r, &q, &q, &r);                    \
            if (!same_le(&q, &LR) || !same_le(&r, &LR2)) bad_alias[idx] = 1;      \
        }

int main(void) {
    CHECK_BIN(0, add)
    CHECK_BIN(1, sub)
    CHECK_BIN(2, mul)
    CHECK_BIN(3, div)
    CHECK_BIN(4, mod)
    CHECK_DIVMOD(5, divmod)
    CHECK_TER(6, addmod)
    CHECK_TER(7, mulmod)
    CHECK_BIN(8, exp)
    CHECK_BIN(9, sdiv)
    CHECK_BIN(10, smod)
    CHECK_DIVMOD(11, sdivmod)
    CHECK_BIN(12, lt)
    CHECK_BIN(13, gt)
    CHECK_BIN(14, slt)
    CHECK_BIN(15, sgt)
    CHECK_BIN(16, eq)
    CHECK_UN(17, iszero)
    CHECK_BIN(18, and)
    CHECK_BIN(19, or)
    CHECK_BIN(20, xor)
    CHECK_UN(21, not)
    CHECK_BIN(22, byte)
    CHECK_BIN(23, shl)
    CHECK_BIN(24, shr)
    CHECK_BIN(25, sar)
    CHECK_BIN(26, signextend)

    /* Negative control: VALS[1] vs VALS[2] must mismatch, VALS[1] vs itself match. */
    load(1, 2, 0);
    uint8_t ctl0 = !same(&LA, &BB), ctl1 = !same(&LA, &BA);

    volatile uint8_t* out = (volatile uint8_t*)0xA0410000ULL;  /* public output */
    uint8_t buf[56];
    for (int i = 0; i < 27; i++) { buf[i] = bad[i]; buf[27 + i] = bad_alias[i]; }
    buf[54] = ctl0; buf[55] = ctl1;
    for (int w = 0; w < 14; w++)
        ((volatile uint32_t*)out)[w] = (uint32_t)buf[4 * w] | ((uint32_t)buf[4 * w + 1] << 8) |
                                      ((uint32_t)buf[4 * w + 2] << 16) |
                                      ((uint32_t)buf[4 * w + 3] << 24);
    return 0;
}
