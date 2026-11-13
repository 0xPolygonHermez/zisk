// zkvm_parallel (zkvm_parallel.h) and the FCSR: every task of a region starts with the FCSR the
// region started with, and after the region it is what the caller left, however the tasks
// changed it (floating-point code rewrites its flags at every operation, and may change its
// rounding mode). The FCSR lives in memory no task owns, so without that a region run in
// parallel would give other traces than one run in order. Emits, as u64 little-endian pairs of
// u32 output slots,
//   [0..TASKS)  the FCSR each task started with
//   [TASKS]     the FCSR after the region
// all of them REGION_FCSR.
#include "zkvm_parallel.h"

#define TASKS 4
#define STACK 4096
#define REGION_FCSR 0x60 // rounding mode 3 (round up), no flags

// One task's arena: what it reports, then its stack, page-aligned
struct arena {
    uint64_t seen;
    uint8_t stack[STACK - 8];
} __attribute__((aligned(4096)));

static struct arena arenas[TASKS];
static uint64_t leave[TASKS] = { 0x1f, 0x80, 0x01, 0xe0 };
static zkvm_task_t tasks[TASKS];

static inline uint64_t read_fcsr(void) {
    uint64_t v;
    __asm__ volatile("csrr %0, 0x003" : "=r"(v));
    return v;
}

static inline void write_fcsr(uint64_t v) {
    __asm__ volatile("csrw 0x003, %0" : : "r"(v));
}

// Reports the FCSR it starts with, then leaves another one
static void task(const void *arg, void *out) {
    *(uint64_t *)out = read_fcsr();
    write_fcsr(*(const uint64_t *)arg);
}

static void emit64(unsigned slot, uint64_t v) {
    volatile uint32_t *o = (volatile uint32_t *)0xA0410000ULL;
    o[2 * slot] = (uint32_t)v;
    o[2 * slot + 1] = (uint32_t)(v >> 32);
}

int main(void) {
    for (unsigned t = 0; t < TASKS; t++) {
        tasks[t] = (zkvm_task_t){
            .entry = task,
            .arg = &leave[t],
            .out = &arenas[t].seen,
            .stack_top = (uint64_t)(uintptr_t)(&arenas[t] + 1),
            .mem_start = (uint64_t)(uintptr_t)&arenas[t],
            .mem_end = (uint64_t)(uintptr_t)(&arenas[t] + 1),
        };
    }
    write_fcsr(REGION_FCSR);

    zkvm_parallel_run(tasks, TASKS);

    for (unsigned t = 0; t < TASKS; t++) emit64(t, arenas[t].seen);
    emit64(TASKS, read_fcsr());
    return 0;
}
