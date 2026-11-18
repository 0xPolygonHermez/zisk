#ifndef EMULATOR_ASM_PAR_HPP
#define EMULATOR_ASM_PAR_HPP

#include <stdbool.h>
#include <stdint.h>

// Fork-join regions in the minimal trace and the memory operations, see par.c; how their chunks
// reach the reader, see par_events.hpp

// Called by the generated assembly, minimal trace and memory operations only, at the FORK and
// BOUNDARY markers. At a BOUNDARY, mem_reads_size is the reads (MT) or records (MO) the step
// wrote (r13) and step_count_down the chunk's count down (r14)
void _par_fork (void);
void _par_boundary (uint64_t mem_reads_size, uint64_t step_count_down);

// The chunk events object of MT and MO (par_events.hpp): created or opened, and mapped, at the
// server's setup, after the output trace; unmapped, and deleted with the output, at its cleanup,
// with the last run's task trace objects
void par_events_setup (void);
void par_events_cleanup (void);

// Before every emulation: a region an aborted emulation left open is stopped (par_reset), the
// previous run's task trace objects go, and the events start over
void par_run_start (void);

// Whether the chunks are published as events (MT and MO), and publishes the chunk the assembly just
// closed: from the main trace, or from the task this process runs (called by _chunk_done)
bool par_events_enabled (void);
void par_chunk_done (void);

// Stops and forgets a region the emulation left open, killing its tasks' processes; nothing in a
// task's process. Called before every emulation (par_run_start), and when one aborts
void par_reset (void);

// Whether this process runs one task of a region for its parent
bool par_is_child (void);

// In a task's process, maps more of its trace object at `address` (trace_map_next_chunk)
void par_child_map_trace (uint64_t address, uint64_t size);

// Ends a task's process without running anything of the server's: its parent reads the exit code
void par_child_exit (int code) __attribute__((noreturn));

// Ends a task's process whose emulation ended, or aborted on a caught signal, before its next
// pass, with the exit code that says why
void par_child_end (bool aborted) __attribute__((noreturn));

// Exit codes of a task's process
#define PAR_EXIT_DONE 0         // reached the next pass, with every chunk published
#define PAR_EXIT_SETUP 2        // could not set up its trace or memory
#define PAR_EXIT_ENDED 3        // the guest ended the emulation inside the task
#define PAR_EXIT_ABORTED 4      // the guest faulted inside the task
#define PAR_EXIT_STATE 5        // reached a pass other than the next one, or in another state
#define PAR_EXIT_PUBLISH 6      // could not publish a chunk or its end
// The host's failures inside the task, not the guest's: they abort the run with PAR_ABORT_ERROR
#define PAR_EXIT_SHM_FULL 7     // writing its trace, an event or the RAM faulted: /dev/shm, which backs them, is full
#define PAR_EXIT_MAX_STEPS 8    // the emulation reached max_steps inside the task
#define PAR_EXIT_RESET 9        // a reset stopped the emulation inside the task: the run is cancelled

// MEM_ERROR of an emulation that a region aborted: above any signal number. The Rust readers
// name each (AsmRunError::from_exit_code), and par_events.rs checks their values against these
#define PAR_ABORT_ERROR 0x100        // a region run apart failed, for a reason that is not the guest's
#define PAR_ABORT_TASK_ENDED 0x101   // the guest ended (it exited or panicked) inside a task
#define PAR_ABORT_TASK_FAULTED 0x102 // the guest faulted inside a task

#endif // EMULATOR_ASM_PAR_HPP
