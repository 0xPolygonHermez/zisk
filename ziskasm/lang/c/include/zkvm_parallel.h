/*
 * zkvm_parallel.h — fork-join regions (ZisK extension, not EF).
 *
 * `zkvm_parallel` runs a list of tasks as if one after another, which is what a
 * proof sees, while the emulator may run them at the same time. It is a zkvmcall
 * thunk in src/zkvm_calls.s (`csrs <id>, x0; ret`), which the transpiler turns into
 * a jump to `ziskasm_zkvm_parallel` in ziskasm/zisklib/zkvm/parallel.zisk. A guest
 * compiles against this header and links `zisklib_c` (see CMakeLists.txt). The Rust
 * binding is `ziskos::par` (and `ziskos::zisklib::zkvm_parallel`, the same ABI).
 *
 * The tasks must be independent:
 * - a task writes only the memory it owns (`mem_start..mem_end`: its stack, its
 *   `out` and anything else it leaves for after the region), and memory nobody
 *   reads afterwards before writing it again: an emulator that runs the tasks apart
 *   keeps only the writes each task makes in the memory it owns;
 * - it must not read what another task of the region writes;
 * - it must not read the input or write outputs (zkvm_io.h), nor use host
 *   randomness or any other host state that the tasks before it would have moved
 *   on (precompiles are fine; an execution with precompile results as hints runs
 *   its regions one task after another);
 * - it must not open a region of its own: regions do not nest;
 * - it must not end the program: a guest that ends (exits or panics) or faults
 *   inside a task fails the execution, with an error that says so, even where its
 *   sequential execution would end normally.
 * Every task starts with the FCSR the region started with, and its rounding-mode
 * changes and floating-point flags end with it: after the region, the FCSR is what
 * the caller left.
 * Data written before the region can be read by every task. Nothing checks this
 * while the guest runs: a region that breaks it gives traces that do not match the
 * sequential execution, and the proof fails.
 */
#ifndef ZKVM_PARALLEL_H
#define ZKVM_PARALLEL_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * One task of a fork-join region, as `zkvm_parallel` reads it: six 64-bit words.
 */
typedef struct zkvm_task {
    /** Runs the task, following the RISC-V calling convention. */
    void (*entry)(const void *arg, void *out);
    /** The task's argument. */
    const void *arg;
    /** Where the task writes its result, within `mem_start..mem_end`. */
    void *out;
    /** The top of the task's own stack, 16-byte aligned, within `mem_start..mem_end`. */
    uint64_t stack_top;
    /**
     * The memory the task owns, `mem_start..mem_end`, both 4096-byte aligned: its
     * stack, its `out` and anything else it writes for after the region. No two
     * tasks of a region may own the same memory.
     */
    uint64_t mem_start;
    uint64_t mem_end;
} zkvm_task_t;

/**
 * Where every task of a region returns. Only `zkvm_parallel` may use it: pass it
 * as its `ret`, never call it.
 */
void zkvm_parallel_ret(void);

/**
 * Run the `n` tasks of `tasks`, in order, each on its own stack. `ret` must be
 * `zkvm_parallel_ret`: each task returns to it, and it leads back into the library
 * routine. Returns once every task is done.
 *
 * @param tasks  `n` tasks, readable by every task of the region
 * @param n      how many tasks
 * @param ret    `zkvm_parallel_ret`
 */
void zkvm_parallel(const zkvm_task_t *tasks, uint64_t n, void (*ret)(void));

/** Run the `n` tasks of `tasks` as one fork-join region. */
static inline void zkvm_parallel_run(const zkvm_task_t *tasks, uint64_t n) {
    zkvm_parallel(tasks, n, zkvm_parallel_ret);
}

#ifdef __cplusplus
}
#endif

#endif /* ZKVM_PARALLEL_H */
