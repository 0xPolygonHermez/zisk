#include "zkvm_accelerators.h"
#include "emit.h"
/* EIP-152 blake2f vector 8 ("abc"): rounds=12, f=1. h/m/t are already in the
 * BLAKE2b little-endian layout the precompile consumes, so no marshalling. */
static uint8_t g_h[64] = {
    0x48,0xc9,0xbd,0xf2,0x67,0xe6,0x09,0x6a, 0x3b,0xa7,0xca,0x84,0x85,0xae,0x67,0xbb,
    0x2b,0xf8,0x94,0xfe,0x72,0xf3,0x6e,0x3c, 0xf1,0x36,0x1d,0x5f,0x3a,0xf5,0x4f,0xa5,
    0xd1,0x82,0xe6,0xad,0x7f,0x52,0x0e,0x51, 0x1f,0x6c,0x3e,0x2b,0x8c,0x68,0x05,0x9b,
    0x6b,0xbd,0x41,0xfb,0xab,0xd9,0x83,0x1f, 0x79,0x21,0x7e,0x13,0x19,0xcd,0xe0,0x5b };
static uint8_t g_m[128] = { 0x61,0x62,0x63 };                 /* "abc", rest zero */
static uint8_t g_t[16]  = { 0x03 };                           /* t = 3            */
static const uint8_t g_ro[32] = {9,8,7,6,5,4,3,2};            /* .rodata non-empty */
static uint8_t g_bss[32];                                     /* .bss non-empty    */
int main(void) {
    __asm__ volatile("" : : "r"(&g_ro[0]), "r"(&g_bss[0]) : "memory");
    zkvm_status s = zkvm_blake2f(12, (zkvm_blake2f_state *)g_h,
                                 (const zkvm_blake2f_message *)g_m,
                                 (const zkvm_blake2f_offset *)g_t, 1);
    g_h[0] ^= (uint8_t)s;                    /* status EOK=0 -> h unchanged */
    emit32(g_h);                             /* first 32 bytes of updated h */
    volatile uint32_t *o = (volatile uint32_t *)(0xA0410000ULL + 32);
    for (unsigned i = 0; i < 8; i++)         /* second 32 bytes at OUTPUT+32 */
        o[i] = (uint32_t)g_h[32+4*i] | ((uint32_t)g_h[32+4*i+1]<<8)
             | ((uint32_t)g_h[32+4*i+2]<<16) | ((uint32_t)g_h[32+4*i+3]<<24);
    return 0;
}
