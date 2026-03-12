#ifndef EMULATOR_ASM_GLOBALS_HPP
#define EMULATOR_ASM_GLOBALS_HPP

#include <stdbool.h>
#include <stdint.h>
#include <time.h>
#include <semaphore.h>
#include <sys/time.h>
#include "constants.hpp"

// Configuration globals, set by arguments
extern bool output;
extern bool output_riscof;
extern bool silent;
extern bool metrics;
extern bool trace;
extern bool trace_trace;
extern bool verbose;
extern bool save_to_file;
extern bool share_input_shm; // Shares input shared memories: input, precompile results and control input, using a common name
extern bool open_input_shm; // Opens existing input shared memories, without creating them.  They must be previously created by another process (assembly emulator or witness computation)
extern char input_file[4096];
extern bool redirect_output_to_file;
extern bool server; // Indicates that this process is a server
extern bool client; // Indicates that this process is a client (used for testing the server)
extern char shm_prefix[MAX_SHM_PREFIX_LENGTH]; // Shared memory prefix
extern int map_locked_flag; // Flag used in mmap to indicate if the physical memory is locked in RAM (MAP_LOCKED) or can be swapped (0).  By default it is locked, but it can be unlocked with the -u argument, which can be useful for testing and debugging purposes, e.g. to allow core dumps when the assembly code crashes
extern uint64_t chunk_mask; // ZIP: 0, 1, 2, 3, 4, 5, 6 or 7
extern bool do_shutdown; // If true, the client will perform a shutdown request to the server when done
extern uint64_t number_of_mt_requests; // Loop to send this number of minimal trace requests
extern uint16_t port; // Service TCP port
extern uint64_t chunk_player_address; // Chunk player address, used for generation methods that use the chunk player, i.e. gen_method=8 or gen_method=10
extern bool wait_flag; // If true, the shmem will get a flag set to 1 if we are waiting for a semaphore, and set it back to 0 when we are not waiting anymore. This can be used for debugging purposes to know if the assembly code is waiting for a semaphore or not.

extern char precompile_file_name[4096]; // Precompile results file name (used by client)
extern char shmem_control_input_name[128];
extern char shmem_control_output_name[128];
extern char shmem_input_name[128];
extern char shmem_output_name[128];
extern char shmem_mt_name[128];
extern char shmem_precompile_name[128];
extern char sem_prec_avail_name[128];
extern char sem_prec_read_name[128];
extern char sem_chunk_done_name[128];
extern char sem_shutdown_done_name[128];
extern char sem_input_avail_name[128];
extern char file_lock_name[128];
extern char log_name[128];
extern bool call_chunk_done;

// Configuration set by assembly code, accessed by C
extern bool precompile_results_enabled;

/*********************/
/* Generation method */
/*********************/

// Specifies how the assembly code generates the trace, and what information it includes.
// It is specified with the mandatory argument --gen=<method>
// It must match the value returned by the assembly function get_gen_method()
// The enum names are equivalent to the rust ones defined in core/src/riscv2zisk.rs as AsmGenerationMethod
// ZisK uses generation methods 1 (minimal trace), 2 (ROM histogram) and 7 (memory operations)
// but the rest of methods can be used for testing and debugging purposes
typedef enum {
    Fast = 0,
    MinimalTrace = 1,
    RomHistogram = 2,
    MainTrace = 3,
    ChunksOnly = 4,
    //BusOp = 5,
    Zip = 6,
    MemOp = 7,
    ChunkPlayerMTCollectMem = 8,
    MemReads = 9,
    ChunkPlayerMemReadsCollectMain = 10,
} GenMethod;

// Default generation method, can be overridden by the --gen argument
extern GenMethod gen_method;

// To be used when calculating partial durations
// Time measurements cannot be overlapped
extern struct timeval start_time;
extern struct timeval stop_time;
extern uint64_t duration;

/*****************/
/* SHARED MEMORY */
/*****************/

// Input shared memory
extern int shmem_input_fd;

// Output trace shared memory
extern int shmem_output_fd;

// Input MT trace shared memory
extern int shmem_mt_fd;

// Chunk done semaphore: notifies the caller when a new chunk has been processed
extern sem_t * sem_chunk_done;

/**************************/
/* PRECOMPILE AND CONTROL */
/**************************/

extern uint64_t * precompile_results_address;

// Precompile results shared memory
extern int shmem_precompile_fd;
extern void * shmem_precompile_address;

// Precompile results semaphores
extern sem_t * sem_prec_avail;
extern sem_t * sem_prec_read;
extern sem_t * sem_input_avail;

// Control input shared memory
extern int shmem_control_input_fd;
extern uint64_t * shmem_control_input_address;
extern volatile uint64_t * precompile_written_address;
extern volatile uint64_t * precompile_exit_address;
extern volatile uint64_t * input_written_address;

// Control output shared memory
extern int shmem_control_output_fd;
extern uint64_t * shmem_control_output_address;
extern volatile uint64_t * precompile_read_address;
extern volatile uint64_t * waiting_for_precompile_address;
extern volatile uint64_t * waiting_for_input_address;

/**************/
/* TRACE SIZE */
/**************/

extern uint64_t initial_trace_size;
extern uint64_t trace_address;
extern uint64_t trace_size;
extern uint64_t trace_used_size;
extern uint64_t trace_address_threshold;

// To be used when calculating the assembly duration
extern uint64_t assembly_duration;

// Counters used in functions called from assembly code
extern uint64_t realloc_counter;
extern uint64_t wait_prec_avail_counter;
extern uint64_t wait_input_avail_counter;
extern uint64_t print_pc_counter;

// Chunk player globals
extern uint64_t chunk_player_mt_size;

// Maximum number of steps to execute, used by the client to limit the execution steps of the
// assembly code.
extern uint64_t max_steps;

// Pointers to the input, RAM, ROM and trace memory, used by both C and assembly code to access these memories
extern uint64_t * pInputTrace; // Used for trace consumption, i.e. chunk player
extern uint64_t * pOutputTrace; // Used for trace generation, i.e. assembly code writes the trace to this address, and client reads it from this address

/**************/
/* CHUNK SIZE */
/**************/

extern uint64_t chunk_size;

#endif // EMULATOR_ASM_GLOBALS_HPP