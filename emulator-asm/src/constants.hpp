#ifndef EMULATOR_ASM_CONSTANTS_HPP
#define EMULATOR_ASM_CONSTANTS_HPP

#include <stdint.h>

/***************/
/* Definitions */
/***************/

// Address map
// There definitions must match the ZisK rust code ones at core/src/mem.rs used to generate the
// assembly code, and that are used by the assembly code to access memory and generate the trace
#define ROM_ADDR (uint64_t)0x80000000
#define ROM_SIZE (uint64_t)0x08000000 // 128MB
#define INPUT_ADDR (uint64_t)0x40000000
#define MAX_INPUT_SIZE (uint64_t)0x40000000 // 1024MB

#define RAM_ADDR (uint64_t)0xA0000000
#define RAM_SIZE (uint64_t)0x20000000 // 512MB
#define SYS_ADDR RAM_ADDR
#define SYS_SIZE (uint64_t)0x10000
#define OUTPUT_ADDR (SYS_ADDR + SYS_SIZE)

#ifdef TRACE_TARGET_MO
    #define TRACE_INITIAL_SIZE (uint64_t)0x180000000 /* 6GB */
    #define TRACE_DELTA_SIZE   (uint64_t)0x080000000 /* 2GB */
#elif defined(TRACE_TARGET_RH)
    #define TRACE_INITIAL_SIZE (uint64_t)0x004000000 /* 64MB */
    #define TRACE_DELTA_SIZE   (uint64_t)0x004000000 /* 64MB */
#else 
    #define TRACE_INITIAL_SIZE (uint64_t)0x180000000 /* 6GB */
    #define TRACE_DELTA_SIZE   (uint64_t)0x080000000 /* 2GB */
#endif

#define TRACE_ADDR         (uint64_t)0xd0000000
#define TRACE_MAX_SIZE     (uint64_t)0x1000000000 // 64GB
#define TRACE_NUMBER_OF_CHUNKS (((TRACE_MAX_SIZE - TRACE_INITIAL_SIZE) / TRACE_DELTA_SIZE) + 1)
#define TRACE_SIZE_GRANULARITY (1014*1014) // ROM histogram trace size is round up to a multiple of this granularity

// Control input and output shared memory configuration.
// Control input is used to tell the assembly code how many precompile result u64 fields have been
// written by the client.  Control output is used to tell the client how many precompile result u64
// fields have been read by the assembly code, so the client can know when it can write new
// precompile results.  Assembly code waits when the number of read fields is not lower than the
// number of written fields, and client waits when the number of written fields would exceed the
// number of read fields plus the available precompile shared memory size, which is a circular buffer
#define CONTROL_INPUT_ADDR (uint64_t)0x70000000
#define CONTROL_INPUT_SIZE (uint64_t)0x1000 // 4kB
#define CONTROL_OUTPUT_ADDR (uint64_t)0x70001000
#define CONTROL_OUTPUT_SIZE (uint64_t)0x1000 // 4kB
#define CONTROL_RETRY_DELAY_US 1000 // 1ms
#define CONTROL_NUMBER_OF_RETRIES 1000 // 1s max total

// Maximum number of steps to execute, used by the client to limit the execution steps of the
// assembly code.  This limit is set by the ZisK PIL constraints.
#define MAX_STEPS (1ULL << 36)

// Assembly service request/response types
// Only the methods supported by the configured generation method will be implemented by the server,
// e.g. gen_method=1 => PING, MT and SHUTDOWN; the rest will fail with an error response.
#define TYPE_PING 1 // Ping
#define TYPE_PONG 2
#define TYPE_MT_REQUEST 3 // Minimal trace
#define TYPE_MT_RESPONSE 4
#define TYPE_RH_REQUEST 5 // ROM histogram
#define TYPE_RH_RESPONSE 6
#define TYPE_MO_REQUEST 7 // Memory opcode
#define TYPE_MO_RESPONSE 8
#define TYPE_MA_REQUEST 9 // Main packed trace
#define TYPE_MA_RESPONSE 10
#define TYPE_CM_REQUEST 11 // Collect memory trace
#define TYPE_CM_RESPONSE 12
#define TYPE_FA_REQUEST 13 // Fast mode, do not generate any trace
#define TYPE_FA_RESPONSE 14
#define TYPE_MR_REQUEST 15 // Mem reads
#define TYPE_MR_RESPONSE 16
#define TYPE_CA_REQUEST 17 // Collect main trace
#define TYPE_CA_RESPONSE 18
#define TYPE_SD_REQUEST 1000000 // Shutdown
#define TYPE_SD_RESPONSE 1000001

// Server IP address, used by the client to connect to the server
#define SERVER_IP "127.0.0.1"  // Change to your server IP; otherwise use localhost IP address

// Chunk size used in generation methods that generate a trace chunk at every N steps, e.g. gen_method=1 or gen_method=7.
// It must be a power of two, and it is used to calculate the trace address threshold at which the next chunk must be mapped,
// to avoid reaching the end of the currently mapped trace memory.
#define CHUNK_SIZE (1ULL << 18)

// Maximum trace chunk size, used to determine when the trace address is close to the end of the
// currently mapped trace memory and the next chunk must be mapped.  It is calculated based on the
// maximum number of bytes that can be generated in a chunk
// Worst case: every chunk instruction is a keccak operation, with an input data of 200 bytes
// (let's use 256 bytes to be safe), and the trace includes the access to 2 source registers, 2
// destination registers and 3 memory addresses (e.g. for a keccak operation with 3 memory operands),
//  which are the maximum number of registers and memory addresses that can be accessed by a chunk
// instruction, according to the ZisK assembly code generation configuration.

#define MAX_MTRACE_REGS_ACCESS_SIZE ((2 + 2 + 3) * 8)
#define MAX_TRACE_CHUNK_INFO ((44*8) + 32)
#define MAX_BYTES_DIRECT_MTRACE 256
#define MAX_BYTES_MTRACE_STEP (MAX_BYTES_DIRECT_MTRACE + MAX_MTRACE_REGS_ACCESS_SIZE)
#define MAX_CHUNK_TRACE_SIZE ((CHUNK_SIZE * MAX_BYTES_MTRACE_STEP) + MAX_TRACE_CHUNK_INFO)

// Maximum precompile results share memory size
// It is a circular buffer
#define MAX_PRECOMPILE_SIZE (uint64_t)0x400000 // 4MB

// Maximum chunk mask for zip generation method, which indicates which chunks are included in the trace,
// and must be between 0 and 7 (inclusive), as it is used to generate a mask of 8 bits where each
// bit indicates if the corresponding chunk is included in the trace or not.
#define MAX_CHUNK_MASK 7

// Maximum length of the shared memory prefix, e.g. "ZISK_12345"
// This prefix is used to generate the names of the shared memories and semaphores used for
// communication and synchronization between the server and the client,
#define MAX_SHM_PREFIX_LENGTH 64

#endif // EMULATOR_ASM_CONSTANTS_HPP