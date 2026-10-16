// zkvm_random_u64 (zkvm_random.h) end-to-end guest: draws N values and emits, as
// u64 little-endian pairs of u32 output slots,
//   [0..20)  the first 20 values (crossing the 17-word SHAKE256 block boundary)
//   [20]     the XOR of all N values
//   [21]     the in-guest checks: 0 = all N values distinct and every bit varies
// The host compares them with SHAKE256 of the seed (see zkvm_random_check.py).
#include "zkvm_random.h"

#define N 500
#define SHOWN 20

static uint64_t vals[N];

static void emit64(unsigned slot, uint64_t v) {
    volatile uint32_t *o = (volatile uint32_t *)0xA0410000ULL;
    o[2 * slot] = (uint32_t)v;
    o[2 * slot + 1] = (uint32_t)(v >> 32);
}

int main(void) {
    uint64_t x = 0, all_or = 0, all_and = ~0ULL, checks = 0;
    for (unsigned i = 0; i < N; i++) {
        uint64_t v = zkvm_random_u64();
        vals[i] = v;
        x ^= v;
        all_or |= v;
        all_and &= v;
    }
    for (unsigned i = 0; i < N; i++)
        for (unsigned j = i + 1; j < N; j++)
            if (vals[i] == vals[j]) checks |= 1;
    if (all_or != ~0ULL || all_and != 0) checks |= 2;
    for (unsigned i = 0; i < SHOWN; i++) emit64(i, vals[i]);
    emit64(SHOWN, x);
    emit64(SHOWN + 1, checks);
    return 0;
}
