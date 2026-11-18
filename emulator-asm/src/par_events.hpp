#ifndef EMULATOR_ASM_PAR_EVENTS_HPP
#define EMULATOR_ASM_PAR_EVENTS_HPP

#include <stdint.h>

/*
 * Chunk events: how a chunked trace service (MT, MO) tells its reader which chunks are ready, so
 * that the chunks of a fork-join region (see par.c) can be read as its tasks produce them, out of
 * order, without being copied into the main trace.
 *
 * Every chunk published, and every region change, appends one event to the service's events
 * object and posts the service's chunk_done semaphore once: the reader handles exactly one event
 * per post, in event order. Must match emulator-asm/asm-runner/src/par_events.rs.
 *
 * Objects, both MAP_SHARED, created by the service (the reader only maps them, read-only):
 *
 * - The events object, <shm_prefix>_<SVC>_events (SVC = MT or MO), PAR_EVENTS_BYTES long, created
 *   at setup. At the start of every run, before the emulation, the service empties (zeroes) the
 *   slots its last run used and then sets `tail` to 0, so a slot the reader is due to read never
 *   holds a previous run's event.
 *
 * - A task's trace object, <sem_prefix>_<SVC>_r<region>_t<task>, PAR_TASK_TRACE_SIZE long
 *   (sparse), created by the parent, every task's of a region, before it publishes REGION. The
 *   task maps it over the whole trace address range, at the same addresses (offset = address -
 *   TRACE_ADDR), so its chunks start at the offset of the chunk the region's first pass opened in
 *   the main trace. `region` counts the regions run apart in one run from 0. Its prefix is the
 *   program's (its semaphores'), not the segments' one that every program of the rank shares, so
 *   that a service never unlinks another program's objects. The reader unlinks each task object
 *   as soon as it maps it, and keeps the mapping until its next run starts, since the chunks it
 *   read borrow them; the service unlinks the objects of its last run that the reader never
 *   mapped at the start of its next run and at its teardown, and at its setup any object of its
 *   program and service that a service which died left behind.
 *
 * Events (a, b, c by kind):
 *
 * - MAIN: the next chunk of the main trace, at the reader's cursor, is complete. Published by the
 *   process that owns the main trace, for every chunk outside regions run apart.
 * - REGION: a region runs apart. a = region, b = tasks, c = the step its first pass starts at.
 *   Published by the parent before it forks any task. The chunk the first pass opened in the main
 *   trace is no main chunk: the main trace continues there after REGION_END, with the chunk of the
 *   region's last pass.
 * - TASK_CHUNK: chunk b (from 0) of task a is complete, at byte offset c of the task's trace
 *   object. Published by the task's process. In MT its header's step counts as if the task ran
 *   first, from the region's first step: the reader adds the steps of the tasks before it.
 * - TASK_DONE: task a reached its next pass after b steps and c chunks (c = b / chunk_size).
 *   Published by the task's process before it exits.
 * - REGION_END: region a is done: every task's process exited after its TASK_DONE, and the main
 *   trace continues at the reader's cursor. Published by the parent.
 *
 * A writer reserves its slot with an atomic fetch-add on `tail`, writes a, b and c, then stores
 * `kind` with release ordering, then posts the semaphore. A reader that finds `kind` still 0 in
 * the slot it is due to read waits for it (its writer is between the two stores).
 *
 * The header also reports, for the reader to read once the service has answered the run, the
 * regions the run ran one task after another and why (a bit per PAR_SEQUENTIAL_* reason), and how
 * a region run apart failed: PAR_FAILURE_*, the region and the task it failed in (PAR_NONE when
 * none), and a detail by failure (see below). The service zeroes them with the slots, and only
 * the process that owns the main trace writes them.
 */

#define PAR_EVENTS_VERSION 3
// 2^18 chunks (the executor's 2^36 steps at the 2^18 steps a chunk the driver pads to), and 2^11
// region events (REGION, TASK_DONE and REGION_END of the 512 tasks a run runs apart at most)
#define PAR_EVENTS_CAPACITY 264192
#define PAR_TASK_TRACE_SIZE (1ULL << 35) // 32 GB, as TRACE_MAX_SIZE

#define PAR_EVENT_MAIN 1
#define PAR_EVENT_REGION 2
#define PAR_EVENT_TASK_CHUNK 3
#define PAR_EVENT_TASK_DONE 4
#define PAR_EVENT_REGION_END 5

typedef struct {
    uint64_t kind; // 0 while its writer fills it, written last
    uint64_t a;
    uint64_t b;
    uint64_t c;
} ParEvent;

// Why a region ran one task after another: the bit (1 << reason) of `sequential_reasons`
#define PAR_SEQUENTIAL_CHUNK_SIZE 1 // the chunk size is not the one the driver pads to
#define PAR_SEQUENTIAL_HINTS 2      // the execution takes precompile results as hints
#define PAR_SEQUENTIAL_WORKERS 3    // one worker: ZISK_ASM_PAR_WORKERS is 1 or less or not a number, or one CPU to run on
#define PAR_SEQUENTIAL_FEW_TASKS 4  // the region has fewer than 2 tasks
#define PAR_SEQUENTIAL_BUDGET 5     // the region would take the run past its budget of tasks run apart
#define PAR_SEQUENTIAL_TASK_LIST 6  // the task list is not what the driver lays out
#define PAR_SEQUENTIAL_FIRST_PASS 7 // the region's first pass does not start a chunk
#define PAR_SEQUENTIAL_HOST 8       // the service could not copy the task list to check it

// How a region run apart failed, and what `failure_detail` is then
#define PAR_FAILURE_TASK_EXIT 1   // a task's process exited with code `detail` (PAR_EXIT_* in par.hpp)
#define PAR_FAILURE_TASK_SIGNAL 2 // a task's process was killed by signal `detail`
#define PAR_FAILURE_TASK_RECORD 3 // a task's process exited without saying how long it ran, or wrongly
#define PAR_FAILURE_MAX_STEPS 4   // the region reaches max_steps; `detail` is the step
#define PAR_FAILURE_HOST 5        // the service could not set the region up; `detail` is errno, or 0
#define PAR_FAILURE_EVENTS 6      // the service could not publish an event; `detail` is errno, or 0
#define PAR_FAILURE_PROTOCOL 7    // the guest broke the driver's protocol (a region inside another, a pass outside one)

// No region, or no task
#define PAR_NONE UINT64_MAX

typedef struct {
    uint64_t version;            // PAR_EVENTS_VERSION
    uint64_t tail;               // Slots reserved in this run
    uint64_t regions_sequential; // Regions this run ran one task after another
    uint64_t sequential_reasons; // Why: a bit (1 << PAR_SEQUENTIAL_*) per reason seen
    uint64_t failure;            // How a region failed this run (PAR_FAILURE_*), 0 if none; written last
    uint64_t failure_region;     // The region run apart it failed in, or PAR_NONE
    uint64_t failure_task;       // The task it failed in, or PAR_NONE
    uint64_t failure_detail;     // By failure, see PAR_FAILURE_*
    ParEvent events[PAR_EVENTS_CAPACITY];
} ParEvents;

#define PAR_EVENTS_BYTES (sizeof(ParEvents))

#endif // EMULATOR_ASM_PAR_EVENTS_HPP
