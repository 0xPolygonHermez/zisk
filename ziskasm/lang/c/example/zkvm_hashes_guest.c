#include "zkvm_accelerators.h"
#include "emit.h"
static uint8_t g_k[32];                            /* .bss    */
static uint8_t g_s[32];
static uint8_t g_in[32] = {1,2,3,4};               /* .data   */
static const uint8_t g_ro[32] = {9,8,7,6,5,4,3,2}; /* .rodata */
int main(void) {
    zkvm_keccak256_hash kh; zkvm_sha256_hash sh;
    zkvm_status s1 = zkvm_keccak256(g_in, 0, &kh);   /* -> ziskasm_zkvm_keccak256 */
    zkvm_status s2 = zkvm_sha256(g_in, 0, &sh);      /* -> ziskasm_zkvm_sha256    */
    __asm__ volatile("" : : "r"(&g_ro[0]) : "memory");
    for (int i = 0; i < 32; i++) { g_k[i] = kh.data[i]; g_s[i] = sh.data[i]; }
    g_k[0] ^= (uint8_t)s1; g_s[0] ^= (uint8_t)s2;    /* status EOK=0 -> unchanged */
    emit32(g_k);
    /* also emit sha to a 2nd 32-byte slot at OUTPUT+32 */
    volatile uint32_t *o = (volatile uint32_t *)(0xA0410000ULL + 32);
    for (unsigned i = 0; i < 8; i++)
        o[i] = (uint32_t)g_s[4*i] | ((uint32_t)g_s[4*i+1]<<8) | ((uint32_t)g_s[4*i+2]<<16) | ((uint32_t)g_s[4*i+3]<<24);
    return 0;
}
