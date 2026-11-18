// zkvm_parallel (zkvm_parallel.h) end-to-end guest: SAXPY, y = a*x + y, over TASKS
// slices of y run as one fork-join region.
//
// It demonstrates the raw ABI, not a speed-up: each task runs about 10^4 steps, and
// the driver pads every task to a chunk boundary (2^18 steps), so the region runs far
// more padding than work and is slower than a sequential loop. A region pays off with
// tasks much longer than a chunk.
//
// A task owns one page-aligned range of memory, mem_start..mem_end, which must hold
// both its stack and everything it writes: run in parallel, a task writes into a
// private copy of the memory outside that range, and those writes are lost. So each
// task's slice of y lives in its arena, next to its stack. x and the arguments are
// written before the region, so every task can read them. A task list that breaks
// these rules (ranges not page-aligned or overlapping, outside the RAM, or a stack
// outside its range) does not fail: the assembly emulator runs the region one task
// after another, with only a warning in its log.
//
// Before the region the guest also computes the expected y, sequentially. After it,
// it compares and emits, as u64 little-endian pairs of u32 output slots,
//   [0]  the sum of y after the region
//   [1]  the in-guest check: 0 = the region's y matches the expected one
#include "zkvm_parallel.h"

#define TASKS 4
#define SLICE 1024 // words of y per task: 8 KiB
#define STACK 8192

// One task's arena, the range it owns: its slice of y, which the task writes, and its
// stack, page-aligned so the tasks own disjoint pages
struct arena {
    uint64_t y[SLICE];
    uint8_t stack[STACK];
} __attribute__((aligned(4096)));

struct saxpy_arg {
    uint64_t a;
    const uint64_t *x; // SLICE words
};

static struct arena arenas[TASKS];
static uint64_t x[TASKS * SLICE];
static uint64_t expected[TASKS * SLICE];
static struct saxpy_arg args[TASKS];
static zkvm_task_t tasks[TASKS];

static void saxpy(const void *arg, void *out) {
    const struct saxpy_arg *s = arg;
    uint64_t *y = out;
    for (unsigned i = 0; i < SLICE; i++) y[i] = s->a * s->x[i] + y[i];
}

static void emit64(unsigned slot, uint64_t v) {
    volatile uint32_t *o = (volatile uint32_t *)0xA0410000ULL;
    o[2 * slot] = (uint32_t)v;
    o[2 * slot + 1] = (uint32_t)(v >> 32);
}

int main(void) {
    const uint64_t a = 3;
    // Before the region: x, the initial y, and the expected result, sequentially
    for (unsigned i = 0; i < TASKS * SLICE; i++) {
        x[i] = i * 7 + 1;
        arenas[i / SLICE].y[i % SLICE] = i;
        expected[i] = a * x[i] + i;
    }
    for (unsigned t = 0; t < TASKS; t++) {
        args[t] = (struct saxpy_arg){ .a = a, .x = &x[t * SLICE] };
        tasks[t] = (zkvm_task_t){
            .entry = saxpy,
            .arg = &args[t],
            .out = arenas[t].y,
            .stack_top = (uint64_t)(uintptr_t)(arenas[t].stack + STACK),
            .mem_start = (uint64_t)(uintptr_t)&arenas[t],
            .mem_end = (uint64_t)(uintptr_t)(&arenas[t] + 1),
        };
    }

    zkvm_parallel_run(tasks, TASKS);

    // After the region: only the comparison
    uint64_t sum = 0, mismatch = 0;
    for (unsigned i = 0; i < TASKS * SLICE; i++) {
        uint64_t y = arenas[i / SLICE].y[i % SLICE];
        sum += y;
        mismatch |= y != expected[i];
    }
    emit64(0, sum);
    emit64(1, mismatch);
    return 0;
}
