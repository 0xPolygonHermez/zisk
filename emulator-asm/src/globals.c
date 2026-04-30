#define _GNU_SOURCE
#include <sys/mman.h>
#include "constants.hpp"
#include "globals.hpp"

// Configuration globals, set by arguments
bool output = false;
bool output_riscof = false;
bool silent = false;
bool metrics = false;
bool trace = false;
bool trace_trace = false;
bool verbose = false;
bool save_to_file = false;
bool share_input_shm = false;
bool create_input_shm = true;
bool create_internal_shm = true;
bool create_output_shm = true;
bool delete_input_shm = true;
bool delete_internal_shm = true;
bool delete_output_shm = true;
bool just_create_all_shm = false;
char input_file[4096] = {0};
bool redirect_output_to_file = false;
bool server = false;
bool client = false;
char shm_prefix[MAX_SHM_PREFIX_LENGTH] = {0};
char sem_prefix[MAX_SHM_PREFIX_LENGTH] = {0};
int map_locked_flag = MAP_LOCKED;
uint64_t chunk_mask = 0x0;
bool do_shutdown = false;
uint64_t number_of_mt_requests = 1;
uint16_t port = 0;
uint64_t chunk_player_address = 0;
bool wait_flag = true;
bool stdio = false;
int server_pid = 0;

// Shared memory names
char shmem_control_input_name[128] = {0};
char shmem_control_output_name[128] = {0};
char shmem_input_name[128] = {0};
char shmem_output_name[128] = {0};
char shmem_mt_name[128] = {0};
char shmem_precompile_name[128] = {0};
char shmem_rom_name[128] = {0};
char shmem_ram_name[128] = {0};

// Semaphore names
char sem_prec_avail_name[128] = {0};
char sem_prec_read_name[128] = {0};
char sem_chunk_done_name[128] = {0};
char sem_shutdown_done_name[128] = {0};
char sem_input_avail_name[128] = {0};

char precompile_file_name[4096] = {0};
char file_lock_name[128] = {0};
char log_name[128] = {0};
bool call_chunk_done = false;

// Configuration set by assembly code, accessed by C
bool precompile_results_enabled = false;

// Default generation method, can be overridden by the --gen argument
GenMethod gen_method = Fast;

// To be used when calculating partial durations
// Time measurements cannot be overlapped
struct timeval start_time;
struct timeval stop_time;
uint64_t duration;

/*****************/
/* SHARED MEMORY */
/*****************/

// Input shared memory
int shmem_input_fd = -1;

// Output trace shared memory
int shmem_output_fd = -1;

// Input MT trace shared memory
int shmem_mt_fd = -1;

// ROM shared memory
int shmem_rom_fd = -1;

// RAM shared memory
int shmem_ram_fd = -1;

// Chunk done semaphore: notifies the caller when a new chunk has been processed
sem_t * sem_chunk_done = NULL;

/**************************/
/* PRECOMPILE AND CONTROL */
/**************************/

uint64_t * precompile_results_address = NULL;

// Precompile results shared memory
int shmem_precompile_fd = -1;
void * shmem_precompile_address = NULL;

// Precompile results semaphores
sem_t * sem_prec_avail = NULL;
sem_t * sem_prec_read = NULL;
sem_t * sem_input_avail = NULL;

// Control input shared memory
int shmem_control_input_fd = -1;
uint64_t * shmem_control_input_address = NULL;
volatile uint64_t * precompile_written_address = NULL;
volatile uint64_t * precompile_exit_address = NULL;
volatile uint64_t * input_written_address = NULL;

// Control output shared memory
int shmem_control_output_fd = -1;
uint64_t * shmem_control_output_address = NULL;
volatile uint64_t * precompile_read_address = NULL;
volatile uint64_t * waiting_for_precompile_address = NULL;
volatile uint64_t * waiting_for_input_address = NULL;

/**************/
/* TRACE SIZE */
/**************/

uint64_t initial_trace_size = TRACE_INITIAL_SIZE;
uint64_t trace_address = TRACE_ADDR;
uint64_t trace_size = TRACE_INITIAL_SIZE;
uint64_t trace_used_size = 0;
uint64_t trace_address_threshold = TRACE_ADDR + TRACE_INITIAL_SIZE - MAX_CHUNK_TRACE_SIZE;

// To be used when calculating the assembly duration
uint64_t assembly_duration;

// Counters used in functions called from assembly code
uint64_t realloc_counter = 0;
uint64_t wait_prec_avail_counter = 0;
uint64_t wait_input_avail_counter = 0;
uint64_t print_pc_counter = 0;

// Chunk player globals
uint64_t chunk_player_mt_size = TRACE_INITIAL_SIZE;

// Maximum number of steps to execute, used by the client to limit the execution steps of the
// assembly code.
uint64_t max_steps = (1ULL << 32);

// Pointers to the input, RAM, ROM and trace memory, used by both C and assembly code to access these memories
uint64_t * pInputTrace = (uint64_t *)TRACE_ADDR; // Used for trace consumption, i.e. chunk player
uint64_t * pOutputTrace = (uint64_t *)TRACE_ADDR; // Used for trace generation, i.e. assembly code writes the trace to this address, and client reads it from this address

/**************/
/* CHUNK SIZE */
/**************/

uint64_t chunk_size = CHUNK_SIZE;