// zkvm_fcall.h end-to-end guest: issues a few hints and emits their results, as
// u64 little-endian pairs of u32 output slots, for the host to compare:
//   [0]      the statuses of three rejected calls (ID 27, ID 0, too many parameters),
//            OR-ed as bits 0..2 when each was ZKVM_EFAIL
//   [1..5)   BN254_FP_INV(2)
//   [5..13)  UINT256_DIV(2^200 + 1000, 7): quotient, remainder
//   [13..18) BIN_DECOMP(0b1011): the length, then the bits
//   [18..23) SECP256K1_FP_SQRT(4, even): is_qr, root
#include "zkvm_fcall.h"

static unsigned slot;

static void emit64(uint64_t v) {
    volatile uint32_t *o = (volatile uint32_t *)0xA0410000ULL;
    o[2 * slot] = (uint32_t)v;
    o[2 * slot + 1] = (uint32_t)(v >> 32);
    slot++;
}

static uint64_t many[ZKVM_FCALL_MAX_PARAMS + 1];

int main(void) {
    uint64_t rejected = 0, out[8];
    const uint64_t two[4] = {2, 0, 0, 0};
    if (zkvm_fcall(27, two, 0) == ZKVM_EFAIL) rejected |= 1;
    if (zkvm_fcall(0, two, 4) == ZKVM_EFAIL) rejected |= 2;
    if (zkvm_fcall(ZKVM_FCALL_BN254_FP_INV, many, ZKVM_FCALL_MAX_PARAMS + 1) == ZKVM_EFAIL)
        rejected |= 4;
    emit64(rejected);

    zkvm_fcall(ZKVM_FCALL_BN254_FP_INV, two, 4);
    zkvm_fcall_read(out, 4);
    for (int i = 0; i < 4; i++) emit64(out[i]);

    const uint64_t ab[8] = {1000, 0, 0, 1ULL << 8, 7, 0, 0, 0};
    zkvm_fcall(ZKVM_FCALL_UINT256_DIV, ab, 8);
    zkvm_fcall_read(out, 8);
    for (int i = 0; i < 8; i++) emit64(out[i]);

    const uint64_t x[2] = {1, 0xb};
    zkvm_fcall(ZKVM_FCALL_BIN_DECOMP, x, 2);
    uint64_t m = zkvm_fcall_get();
    emit64(m);
    for (uint64_t i = 0; i < m; i++) emit64(zkvm_fcall_get());

    const uint64_t four[5] = {4, 0, 0, 0, 0};
    zkvm_fcall(ZKVM_FCALL_SECP256K1_FP_SQRT, four, 5);
    zkvm_fcall_read(out, 5);
    for (int i = 0; i < 5; i++) emit64(out[i]);
    return 0;
}
