/* Test adapter around the unmodified, bundled Berkeley SoftFloat-3e library: the
 * FP16-input multiply composite f32_mul(f16_to_f32(x), f16_to_f32(y)) with all flags.
 * Its sources retain the Regents of the University of California license. */
#include <inttypes.h>
#include <stdio.h>
#include "softfloat.h"

static uint32_t canonical(uint32_t bits) {
    return (bits & 0x7fffffff) > 0x7f800000 ? 0x7fc00000 : bits;
}

int main(void) {
    unsigned a, b;
    softfloat_roundingMode = softfloat_round_near_even;
    softfloat_detectTininess = softfloat_tininess_afterRounding;
    int fields;
    while ((fields = scanf("%x %x", &a, &b)) == 2) {
        softfloat_exceptionFlags = 0;
        float32_t x = f16_to_f32((float16_t){(uint16_t)a});
        float32_t y = f16_to_f32((float16_t){(uint16_t)b});
        uint32_t product = canonical(f32_mul(x, y).v);
        printf("%08" PRIx32 " %02x\n", product, (unsigned)softfloat_exceptionFlags);
    }
    return fields == EOF ? 0 : 1;
}
