#include "runtime.h"
#include "zkvm_accelerators.h"

_Noreturn void runtime_trap(void) {
    __asm__ volatile("ebreak");
    __builtin_unreachable();
}

uint8_t *runtime_checked_range(uint64_t offset, uint64_t len, uint64_t mem_end) {
    if (offset > UINT32_MAX || len > UINT32_MAX || mem_end < NITRO_WASM_BASE ||
        offset + len > mem_end - NITRO_WASM_BASE)
        runtime_trap();
    return (uint8_t *)(uintptr_t)(NITRO_WASM_BASE + offset);
}

void runtime_keccak(const void *input, size_t len, uint8_t output[32]) {
    zkvm_keccak256_hash digest;
    if (zkvm_keccak256(input, len, &digest) != ZKVM_EOK) runtime_trap();
    memcpy(output, digest.data, 32);
}

void runtime_sha256(const void *input, size_t len, uint8_t output[32]) {
    zkvm_sha256_hash digest;
    if (zkvm_sha256(input, len, &digest) != ZKVM_EOK) runtime_trap();
    memcpy(output, digest.data, 32);
}

uint64_t nitro_dispatch(uint64_t op, const uint64_t args[8], uint64_t mem_end) {
    if (op == 20 || op == 21) return runtime_brotli(op, args, mem_end);
    return runtime_state(op, args, mem_end);
}
