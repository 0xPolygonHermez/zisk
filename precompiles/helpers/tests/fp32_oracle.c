/* Test adapter around the unmodified, bundled Berkeley SoftFloat-3e library.
 * Its sources retain the Regents of the University of California license. */
#include <inttypes.h>
#include <stdio.h>
#include "softfloat.h"

static uint32_t canonical(uint32_t bits) {
    return (bits & 0x7fffffff) > 0x7f800000 ? 0x7fc00000 : bits;
}

int main(void) {
    uint32_t a, b;
    softfloat_roundingMode = softfloat_round_near_even;
    softfloat_detectTininess = softfloat_tininess_afterRounding;
    int fields;
    while ((fields = scanf("%" SCNx32 " %" SCNx32, &a, &b)) == 2) {
        softfloat_exceptionFlags = 0;
        uint32_t sum = canonical(f32_add((float32_t){a}, (float32_t){b}).v);
        unsigned add_flags = softfloat_exceptionFlags;
        softfloat_exceptionFlags = 0;
        uint32_t product = canonical(f32_mul((float32_t){a}, (float32_t){b}).v);
        printf("%08" PRIx32 " %02x %08" PRIx32 " %02x\n",
               sum, add_flags, product, (unsigned)softfloat_exceptionFlags);
    }
    return fields == EOF ? 0 : 1;
}
