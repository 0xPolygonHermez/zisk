#include "zkvm_accelerators.h"
#include "emit.h"
static uint8_t g_in[8] = {0};
static uint8_t g_pad[32] = {1,2,3,4};
static volatile uint32_t s = 1;
int main(void) {
    zkvm_keccak256_hash out;
    zkvm_status st = zkvm_keccak256(g_in, 0, &out);   // EF ABI -> ziskos_keccak -> .zisk
    if (st != ZKVM_EOK) __asm__ volatile("unimp");
    emit32(out.data);
    s = out.data[0] + g_pad[0];
    return 0;
}
