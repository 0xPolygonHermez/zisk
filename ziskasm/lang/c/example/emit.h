#include <stdint.h>
static inline void emit32(const uint8_t b[32]) {
    volatile uint32_t *o = (volatile uint32_t *)0xA0410000ULL;
    for (unsigned i = 0; i < 8; i++)
        o[i] = (uint32_t)b[4*i] | ((uint32_t)b[4*i+1]<<8) | ((uint32_t)b[4*i+2]<<16) | ((uint32_t)b[4*i+3]<<24);
}
