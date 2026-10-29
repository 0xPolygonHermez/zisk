// zkvm_blake2f (zkvm_accelerators.h) final-flag guest: EIP-152 accepts only f = 0 or 1,
// and any other flag must return ZKVM_EFAIL with h untouched. Each case starts from the
// same h (EIP-152 vector 8, "abc") and emits two u32 output slots,
//   [2i]      the returned zkvm_status (EOK = 0, EFAIL = 0xffffffff)
//   [2i + 1]  1 if h is still the initial state, else 0
// for the cases below, in order. The host checks them (emulator/tests/blake2f_flag.rs).
#include "zkvm_accelerators.h"

static const uint8_t H0[64] = {
    0x48,0xc9,0xbd,0xf2,0x67,0xe6,0x09,0x6a, 0x3b,0xa7,0xca,0x84,0x85,0xae,0x67,0xbb,
    0x2b,0xf8,0x94,0xfe,0x72,0xf3,0x6e,0x3c, 0xf1,0x36,0x1d,0x5f,0x3a,0xf5,0x4f,0xa5,
    0xd1,0x82,0xe6,0xad,0x7f,0x52,0x0e,0x51, 0x1f,0x6c,0x3e,0x2b,0x8c,0x68,0x05,0x9b,
    0x6b,0xbd,0x41,0xfb,0xab,0xd9,0x83,0x1f, 0x79,0x21,0x7e,0x13,0x19,0xcd,0xe0,0x5b };
static uint8_t g_m[128] = { 0x61,0x62,0x63 };                 /* "abc", rest zero */
static uint8_t g_t[16]  = { 0x03 };                           /* t = 3            */
static uint8_t g_h[64];

static const struct { uint32_t rounds; uint8_t f; } CASES[] = {
    { 12, 0 },     /* valid: EOK, h updated */
    { 12, 1 },     /* valid: EOK, h updated */
    { 12, 2 },     /* invalid: EFAIL, h untouched */
    { 12, 0x80 },
    { 12, 0xff },
    { 0, 2 },      /* invalid with no rounds: the check comes before the rounds */
};

int main(void) {
    volatile uint32_t *o = (volatile uint32_t *)0xA0410000ULL;
    for (unsigned i = 0; i < sizeof CASES / sizeof CASES[0]; i++) {
        for (unsigned k = 0; k < 64; k++) g_h[k] = H0[k];
        zkvm_status s = zkvm_blake2f(CASES[i].rounds, (zkvm_blake2f_state *)g_h,
                                     (const zkvm_blake2f_message *)g_m,
                                     (const zkvm_blake2f_offset *)g_t, CASES[i].f);
        uint32_t untouched = 1;
        for (unsigned k = 0; k < 64; k++)
            if (g_h[k] != H0[k]) untouched = 0;
        o[2 * i] = (uint32_t)s;
        o[2 * i + 1] = untouched;
    }
    return 0;
}
