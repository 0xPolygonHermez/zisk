#ifndef NITRO_RUNTIME_H
#define NITRO_RUNTIME_H
#include <stddef.h>
#include <stdint.h>

#define NITRO_WASM_BASE UINT64_C(0xa1000000)
#define NITRO_INPUT_DATA UINT64_C(0x10000010)
#define NITRO_OUTPUT UINT64_C(0xa0410000)
#define NITRO_MODULE_HASH UINT64_C(0xa0540300)

_Noreturn void runtime_trap(void);
uint8_t *runtime_checked_range(uint64_t offset, uint64_t len, uint64_t mem_end);
void runtime_keccak(const void *input, size_t len, uint8_t output[32]);
void runtime_sha256(const void *input, size_t len, uint8_t output[32]);
uint64_t runtime_state(uint64_t op, const uint64_t args[8], uint64_t mem_end);
uint64_t runtime_brotli(uint64_t op, const uint64_t args[8], uint64_t mem_end);
uint64_t nitro_dispatch(uint64_t op, const uint64_t args[8], uint64_t mem_end);

void *memcpy(void *dst, const void *src, size_t len);
void *memset(void *dst, int value, size_t len);
int memcmp(const void *a, const void *b, size_t len);
#endif
