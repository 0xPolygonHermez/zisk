/* unaligned_out.c -- ZisK library functions called with their result buffer at
   byte offsets 0..7. The precompiles need 8-byte aligned pointers, so a routine
   that hands the caller's result pointer to one must detect an unaligned result
   (`unal8`) and compute into aligned scratch instead. Each call must write the
   same bytes as a call with an aligned result, and touch nothing around them.
   Output: per case, one byte with bit `o` set when offset `o` is right (0xff when
   all eight are). check.sh compares it with a golden snapshot. */
#include <stddef.h>
#include <stdint.h>

#include "zkvm_zisklib.h"

static volatile uint8_t* const O = (volatile uint8_t*)0xA0410000ULL;

static const uint64_t A[4] = {0x0123456789abcdef, 0xfedcba9876543210, 0x0f1e2d3c4b5a6978, 0x1122334455667788};
static const uint64_t B[4] = {0x8877665544332211, 0x0011223344556677, 0x99aabbccddeeff00, 0x0000000000000abc};
static const uint64_t M[4] = {0xffffffff00000001, 0x0000000000000000, 0x00000000ffffffff, 0xffffffff00000000};

/* Each case writes its result(s) to `out`; `len` is the bytes it writes. */
typedef void (*case_fn)(uint64_t* out);
static void c_reduce_mod256(uint64_t* out) { zkvm_zisklib_reduce_mod256(A, M, out); }
static void c_add_mod256(uint64_t* out) { zkvm_zisklib_add_mod256(A, B, M, out); }
static void c_mul_mod256(uint64_t* out) { zkvm_zisklib_mul_mod256(A, B, M, out); }

static const struct {
    case_fn f;
    size_t len;
} CASES[] = {
    {c_reduce_mod256, 32},
    {c_add_mod256, 32},
    {c_mul_mod256, 32},
};

#define MAX_LEN 256
#define GUARD 8
#define FILL 0xA5

static uint64_t ref[MAX_LEN / 8];
static uint64_t buf[(MAX_LEN + 2 * GUARD) / 8];

int main(void) {
    for (size_t c = 0; c < sizeof(CASES) / sizeof(CASES[0]); c++) {
        size_t len = CASES[c].len;
        CASES[c].f(ref);
        const uint8_t* e = (const uint8_t*)ref;
        uint8_t mask = 0;
        for (int o = 0; o < 8; o++) {
            uint8_t* b = (uint8_t*)buf;
            for (size_t i = 0; i < sizeof(buf); i++) b[i] = FILL;
            CASES[c].f((uint64_t*)(b + GUARD + o));
            int ok = 1;
            for (size_t i = 0; i < sizeof(buf); i++) {
                size_t lo = GUARD + o;
                uint8_t want = (i >= lo && i < lo + len) ? e[i - lo] : FILL;
                if (b[i] != want) ok = 0;
            }
            mask |= (uint8_t)(ok << o);
        }
        O[c] = mask;
    }
    return 0;
}
