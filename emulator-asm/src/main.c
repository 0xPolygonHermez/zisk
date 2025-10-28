#define _GNU_SOURCE
#include <stdio.h>
#include <sys/mman.h>
#include <bits/mman-linux.h>
#include <stdbool.h>
#include <string.h>
#include <stdint.h>
#include <sys/time.h>
#include <bits/mman-shared.h>
#include <stdlib.h>
#include <errno.h>
#include <fcntl.h>
#include <unistd.h>
#include <assert.h>
#include <semaphore.h>
#include "../../lib-c/c/src/ec/ec.hpp"
#include "../../lib-c/c/src/fcall/fcall.hpp"
#include "../../lib-c/c/src/arith256/arith256.hpp"
#include "emu.hpp"
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netinet/tcp.h>

// Assembly-provided functions
void emulator_start(void);
void write_ro_data(void);
uint64_t get_max_bios_pc(void);
uint64_t get_max_program_pc(void);
uint64_t get_gen_method(void);

// Address map
#define ROM_ADDR (uint64_t)0x80000000
#define ROM_SIZE (uint64_t)0x08000000 // 128MB

#define INPUT_ADDR (uint64_t)0x90000000
#define MAX_INPUT_SIZE (uint64_t)0x08000000 // 128MB

#define RAM_ADDR (uint64_t)0xa0000000
#define RAM_SIZE (uint64_t)0x20000000 // 512MB
#define SYS_ADDR RAM_ADDR
#define SYS_SIZE (uint64_t)0x10000
#define OUTPUT_ADDR (SYS_ADDR + SYS_SIZE)

#define TRACE_ADDR         (uint64_t)0xc0000000
#define INITIAL_TRACE_SIZE (uint64_t)0x100000000 // 4GB

#define REG_ADDR (uint64_t)0x70000000
#define REG_SIZE (uint64_t)0x1000 // 4kB

uint8_t * pInput = (uint8_t *)INPUT_ADDR;
uint8_t * pInputLast = (uint8_t *)(INPUT_ADDR + 10440504 - 64);
uint8_t * pRam = (uint8_t *)RAM_ADDR;
uint8_t * pRom = (uint8_t *)ROM_ADDR;
uint64_t * pInputTrace = (uint64_t *)TRACE_ADDR;
uint64_t * pOutputTrace = (uint64_t *)TRACE_ADDR;

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

// Generation method
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
GenMethod gen_method = Fast;

// Service TCP parameters
#define SERVER_IP "127.0.0.1"  // Change to your server IP
uint16_t port = 0;
uint16_t arguments_port = 0;

// Type of execution
bool server = false;
bool client = false;
bool call_chunk_done = false;
bool do_shutdown = false; // If true, the client will perform a shutdown request to the server when done
uint64_t number_of_mt_requests = 1; // Loop to send this number of minimal trace requests

char input_file[4096];

// To be used when calculating partial durations
// Time measurements cannot be overlapped
struct timeval start_time;
struct timeval stop_time;
uint64_t duration;

// To be used when calculating total duration
struct timeval total_start_time;
struct timeval total_stop_time;
uint64_t total_duration;

// To be used when calculating the assembly duration
uint64_t assembly_duration;

extern uint64_t MEM_STEP;
extern uint64_t MEM_END;
extern uint64_t MEM_ERROR;
extern uint64_t MEM_TRACE_ADDRESS;
extern uint64_t MEM_CHUNK_ADDRESS;
extern uint64_t MEM_CHUNK_START_STEP;

uint64_t realloc_counter = 0;

extern void zisk_keccakf(uint64_t state[25]);
/* Used for debugging
extern uint64_t reg_0;
extern uint64_t reg_1;
extern uint64_t reg_2;
extern uint64_t reg_3;
extern uint64_t reg_4;
extern uint64_t reg_5;
extern uint64_t reg_6;
extern uint64_t reg_7;
extern uint64_t reg_8;
extern uint64_t reg_9;
extern uint64_t reg_10;
extern uint64_t reg_11;
extern uint64_t reg_12;
extern uint64_t reg_13;
extern uint64_t reg_14;
extern uint64_t reg_15;
extern uint64_t reg_16;
extern uint64_t reg_17;
extern uint64_t reg_18;
extern uint64_t reg_19;
extern uint64_t reg_20;
extern uint64_t reg_21;
extern uint64_t reg_22;
extern uint64_t reg_23;
extern uint64_t reg_24;
extern uint64_t reg_25;
extern uint64_t reg_26;
extern uint64_t reg_27;
extern uint64_t reg_28;
extern uint64_t reg_29;
extern uint64_t reg_30;
extern uint64_t reg_31;
*/

bool is_power_of_two (uint64_t number) {
    return (number != 0) && ((number & (number - 1)) == 0);
}

#define INITIAL_CHUNK_SIZE (1ULL << 18)
uint64_t chunk_size = INITIAL_CHUNK_SIZE;
uint64_t chunk_size_mask = INITIAL_CHUNK_SIZE - 1;
uint64_t max_steps = (1ULL << 32);

// Chunk player globals
uint64_t chunk_player_address = 0;
uint64_t chunk_player_mt_size = INITIAL_TRACE_SIZE; // TODO

void set_max_steps (uint64_t new_max_steps)
{
    if (!is_power_of_two(new_max_steps))
    {
        printf("ERROR: set_max_steps() got a new max steps = %lu that is not a power of two\n", new_max_steps);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    max_steps = new_max_steps;
}

uint64_t initial_trace_size = INITIAL_TRACE_SIZE;
uint64_t trace_address = TRACE_ADDR;
uint64_t trace_size = INITIAL_TRACE_SIZE;
uint64_t trace_used_size = 0;

// Worst case: every chunk instruction is a keccak operation, with an input data of 200 bytes
#define MAX_CHUNK_TRACE_SIZE (INITIAL_CHUNK_SIZE * 200) + (44 * 8) + 32
uint64_t trace_address_threshold = TRACE_ADDR + INITIAL_TRACE_SIZE - MAX_CHUNK_TRACE_SIZE;

uint64_t print_pc_counter = 0;

int map_locked_flag = MAP_LOCKED;

#ifdef ASM_PRECOMPILE_CACHE
bool precompile_cache_enabled = false;
#endif

void set_chunk_size (uint64_t new_chunk_size)
{
    if (!is_power_of_two(new_chunk_size))
    {
        printf("ERROR: set_chunk_size() got a new chunk size = %lu that is not a power of two\n", new_chunk_size);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    chunk_size = new_chunk_size;
    chunk_size_mask = chunk_size - 1;
    trace_address_threshold = TRACE_ADDR + trace_size - ((chunk_size*200) + (44*8) + 32);
}

void set_trace_size (uint64_t new_trace_size)
{
    // Update trace global variables
    trace_size = new_trace_size;
    trace_address_threshold = TRACE_ADDR + trace_size - MAX_CHUNK_TRACE_SIZE;
    pOutputTrace[2] = trace_size;
}

void parse_arguments(int argc, char *argv[]);
uint64_t TimeDiff(const struct timeval startTime, const struct timeval endTime);

void configure (void);
void server_setup (void);
void server_reset (void);
void server_run (void);
void server_cleanup (void);

void client_setup (void);
void client_run (void);
void client_cleanup (void);

void _chunk_done(void);

void log_minimal_trace(void);
void log_histogram(void);
void log_main_trace(void);
void log_mem_trace(void);
void log_mem_op(void);
void save_mem_op_to_files(void);
void log_chunk_player_main_trace(void);

int recv_all_with_timeout (int sockfd, void *buffer, size_t length, int flags, int timeout_sec);

// Configuration
bool output = false;
bool silent = false;
bool metrics = false;
bool trace = false;
bool trace_trace = false;
bool verbose = false;
bool save_to_file = false;

// ROM histogram
uint64_t histogram_size = 0;
uint64_t bios_size = 0;
uint64_t program_size = 0;

// Zip
uint64_t chunk_mask = 0x0; // 0, 1, 2, 3, 4, 5, 6 or 7
#define MAX_CHUNK_MASK 7

// Maximum length of the shared memory prefix, e.g. SHMZISK12345678
#define MAX_SHM_PREFIX_LENGTH 64
char shm_prefix[MAX_SHM_PREFIX_LENGTH];

// Input shared memory
char shmem_input_name[128];
int shmem_input_fd = -1;
uint64_t shmem_input_size = 0;
void * shmem_input_address = NULL;

// Output trace shared memory
char shmem_output_name[128];
int shmem_output_fd = -1;

// Input MT trace shared memory
char shmem_mt_name[128];
int shmem_mt_fd = -1;

// Chunk done semaphore: notifies the caller when a new chunk has been processed
char sem_chunk_done_name[128];
sem_t * sem_chunk_done = NULL;

// Shutdown done semaphore: notifies the caller when a shutdown has been processed
char sem_shutdown_done_name[128];
sem_t * sem_shutdown_done = NULL;

int process_id = 0;

uint64_t input_size = 0;

int main(int argc, char *argv[])
{
#ifdef DEBUG
    // Start counting total execution time
    gettimeofday(&total_start_time, NULL);
#endif

    // Result, to be used in calls to functions returning int
    int result;

    // Get current process id
    process_id = getpid();

    // Parse arguments
    parse_arguments(argc, argv);

    // Configure based on parguments
    configure();

    // If this is a client, run it and quit
    if (client)
    {
        // Setup the client
        client_setup();

        // Run the client
        client_run();

        // Cleanup the client
        client_cleanup();

        return 0;
    }

    // Setup the server
    server_setup();

    // Reset the server, i.e. reset memory
    server_reset();

    // Create socket file descriptor
    int server_fd;
    server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd == 0)
    {
        printf("ERROR: Failed calling socket() errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Forcefully attach socket to the port (avoid "address already in use")
    int opt = 1;
    result = setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR | SO_REUSEPORT, &opt, sizeof(opt));
    if (result != 0)
    {
        printf("ERROR: Failed calling setsockopt() result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    struct sockaddr_in address;
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = INADDR_ANY; // Listen on all interfaces
    address.sin_port = htons(port);

    // Bind socket to port
    result = bind(server_fd, (struct sockaddr *)&address, sizeof(address));
    if (result != 0)
    {
        printf("ERROR: Failed calling bind() result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Start listening
    result = listen(server_fd, 5);
    if (result != 0)
    {
        printf("ERROR: Failed calling listen() result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    while (true)
    {
        // Accept incoming connection
        struct sockaddr_in address;
        int addrlen = sizeof(address);
        int client_fd;
        if (!silent) printf("Waiting for incoming connections to port %u...\n", port);
        client_fd = accept(server_fd, (struct sockaddr *)&address, (socklen_t*)&addrlen);
        if (client_fd < 0)
        {
            printf("ERROR: Failed calling accept() client_fd=%d errno=%d=%s\n", client_fd, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
#ifdef DEBUG
        if (verbose) printf("New client: %s:%d\n", inet_ntoa(address.sin_addr), ntohs(address.sin_port));
#endif

        // Configure linger to send data before closing the socket
        // struct linger linger_opt = {1, 5};  // Enable linger with 5s timeout
        // setsockopt(client_fd, SOL_SOCKET, SO_LINGER, &linger_opt, sizeof(linger_opt));
        // int cork = 0;
        // setsockopt(client_fd, IPPROTO_TCP, TCP_CORK, &cork, sizeof(cork));
        // // Disable Nagle algorithm
        // int flag = 1;
        // setsockopt(client_fd, IPPROTO_TCP, TCP_NODELAY, &flag, sizeof(flag));

        bool bShutdown = false;
        bool bReset;

        while (true)
        {
            // Read client request
            uint64_t request[5];
            ssize_t bytes_read = recv(client_fd, request, sizeof(request), MSG_WAITALL);
            if (bytes_read < 0)
            {
                printf("ERROR: Failed calling recv() bytes_read=%ld errno=%d=%s\n", bytes_read, errno, strerror(errno));
                break;
            }
            if (bytes_read != sizeof(request))
            {
                if ((errno != 0) && (errno != 2))
                {
                    printf("ERROR: Failed calling recv() invalid bytes_read=%ld errno=%d=%s\n", bytes_read, errno, strerror(errno));
                }
                break;
            }
#ifdef DEBUG
            if (verbose) printf("recv() returned: %ld\n", bytes_read);
#endif
            if (verbose) printf("recv()'d request=[%lu, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n", request[0], request[1], request[2], request[3], request[4]);

            uint64_t response[5];
            bReset = false;
            switch (request[0])
            {
                case TYPE_PING:
                {
#ifdef DEBUG
                    if (verbose) printf("PING received\n");
#endif
                    response[0] = TYPE_PONG;
                    response[1] = gen_method;
                    response[2] = trace_size;
                    response[3] = 0;
                    response[4] = 0;
                    break;
                }
                case TYPE_MT_REQUEST:
                {
#ifdef DEBUG
                    if (verbose) printf("MINIMAL TRACE received\n");
#endif
                    if (gen_method == MinimalTrace)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        response[0] = TYPE_MT_RESPONSE;
                        response[1] = (MEM_END && !MEM_ERROR) ? 0 : 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;

                        bReset = true;
                    }
                    else
                    {
                        response[0] = TYPE_MT_RESPONSE;
                        response[1] = 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;
                    }
                    break;
                }
                case TYPE_RH_REQUEST:
                {
#ifdef DEBUG
                    if (verbose) printf("ROM HISTOGRAM received\n");
#endif
                    if (gen_method == RomHistogram)
                    {
                        set_max_steps(request[1]);

                        server_run();

                        response[0] = TYPE_RH_RESPONSE;
                        response[1] = MEM_END ? 0 : 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;

                        bReset = true;
                    }
                    else
                    {
                        response[0] = TYPE_RH_RESPONSE;
                        response[1] = 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;
                    }
                    break;
                }
                case TYPE_MO_REQUEST:
                {
#ifdef DEBUG
                    if (verbose) printf("MEMORY OPERATIONS received\n");
#endif
                    if (gen_method == MemOp)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        response[0] = TYPE_MO_RESPONSE;
                        response[1] = MEM_END ? 0 : 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;

                        bReset = true;
                    }
                    else
                    {
                        response[0] = TYPE_MO_RESPONSE;
                        response[1] = 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;
                    }
                    break;
                }
                case TYPE_MA_REQUEST:
                {
#ifdef DEBUG
                    if (verbose) printf("MAIN TRACE received\n");
#endif
                    if (gen_method == MainTrace)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        response[0] = TYPE_MA_RESPONSE;
                        response[1] = MEM_END ? 0 : 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;

                        bReset = true;
                    }
                    else
                    {
                        response[0] = TYPE_MA_RESPONSE;
                        response[1] = 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;
                    }
                    break;
                }
                case TYPE_CM_REQUEST:
                {
#ifdef DEBUG
                    if (verbose) printf("COLLECT MEMORY received\n");
#endif
                    if (gen_method == ChunkPlayerMTCollectMem)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);
                        chunk_player_address = request[3];
                        uint64_t * pChunk = (uint64_t *)chunk_player_address;
                        print_pc_counter = pChunk[3];

                        server_run();

                        response[0] = TYPE_CM_RESPONSE;
                        response[1] = 0;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;

                        bReset = true;
                    }
                    else
                    {
                        response[0] = TYPE_CM_RESPONSE;
                        response[1] = 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;
                    }
                    break;
                }
                case TYPE_FA_REQUEST:
                {
#ifdef DEBUG
                    if (verbose) printf("FAST received\n");
#endif
                    if (gen_method == Fast)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        response[0] = TYPE_FA_RESPONSE;
                        response[1] = MEM_END ? 0 : 1;
                        response[2] = 0;
                        response[3] = 0;
                        response[4] = 0;

                        bReset = true;
                    }
                    else
                    {
                        response[0] = TYPE_FA_RESPONSE;
                        response[1] = 1;
                        response[2] = 0;
                        response[3] = 0;
                        response[4] = 0;
                    }
                    break;
                }
                case TYPE_MR_REQUEST:
                {
#ifdef DEBUG
                    if (verbose) printf("MEMORY READS received\n");
#endif
                    if (gen_method == MemReads)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        response[0] = TYPE_MR_RESPONSE;
                        response[1] = MEM_END ? 0 : 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;

                        bReset = true;
                    }
                    else
                    {
                        response[0] = TYPE_MR_RESPONSE;
                        response[1] = 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;
                    }
                    break;
                }
                case TYPE_CA_REQUEST:
                {
#ifdef DEBUG
                    if (verbose) printf("COLLECT MAIN received\n");
#endif
                    if (gen_method == ChunkPlayerMemReadsCollectMain)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);
                        chunk_player_address = request[3];
                        uint64_t * pChunk = (uint64_t *)chunk_player_address;
                        print_pc_counter = pChunk[3];

                        server_run();

                        response[0] = TYPE_CA_RESPONSE;
                        response[1] = 0;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;

                        bReset = true;
                    }
                    else
                    {
                        response[0] = TYPE_CA_RESPONSE;
                        response[1] = 1;
                        response[2] = trace_size;
                        response[3] = trace_used_size;
                        response[4] = 0;
                    }
                    break;
                }
                case TYPE_SD_REQUEST:
                {
                    if (!silent) printf("SHUTDOWN received\n");
                    bShutdown = true;

                    response[0] = TYPE_SD_RESPONSE;
                    response[1] = 0;
                    response[2] = 0;
                    response[3] = 0;
                    response[4] = 0;
                    break;
                }
                default:
                {
                    printf("ERROR: Invalid request id=%lu\n", request[0]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);                
                }
            }

            if (verbose) printf("send()'ing response=[%lu, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n", response[0], response[1], response[2], response[3], response[4]);

            ssize_t bytes_sent = send(client_fd, response, sizeof(response), MSG_WAITALL);
            if (bytes_sent != sizeof(response))
            {
                printf("ERROR: Failed calling send() invalid bytes_sent=%ld errno=%d=%s\n", bytes_sent, errno, strerror(errno));
                break;
            }
#ifdef DEBUG
            else if (verbose) printf("Response sent to client\n");
#endif
            if (bReset)
            {
                server_reset();
            }

            if (bShutdown)
            {
                break;
            }
        }

        // Chutdown the client socket
        shutdown(client_fd, SHUT_WR);

        // Close client socket
        close(client_fd);

        if (bShutdown)
        {
            break;
        }
    }

    // Close the server
    close(server_fd);


    /************/
    /* CLEAN UP */
    /************/

    server_cleanup();

#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_enabled)
    {
        precompile_cache_cleanup();
    }
#endif

    fflush(stdout);
    fflush(stderr);

    #ifdef DEBUG
        gettimeofday(&total_stop_time, NULL);
        total_duration = TimeDiff(total_start_time, total_stop_time);
        uint64_t assembly_percentage = total_duration == 0 ? 0 : assembly_duration * 1000 / total_duration;
        if (verbose) printf("Emulator C end total_duration = %lu us assembly_duration = %lu us (%lu %%o)\n", total_duration, assembly_duration, assembly_percentage);
    #endif
}

void print_usage (void)
{
    printf("Usage: ziskemuasm\n");
    printf("\t-s(server)\n");
    printf("\t-c(client)\n");
    printf("\t-i <input_file>\n");
    printf("\t-p <port_number>\n");
    printf("\t--gen=0|--generate_fast\n");
    printf("\t--gen=1|--generate_minimal_trace\n");
    printf("\t--gen=2|--generate_rom_histogram\n");
    printf("\t--gen=3|--generate_main_trace\n");
    printf("\t--gen=4|--generate_chunks\n");
    printf("\t--gen=6|--generate_zip\n");
    printf("\t--gen=9|--generate_mem_reads\n");
    printf("\t--gen=10|--generate_chunk_player_mem_reads\n");
    printf("\t--chunk <chunk_number>\n");
    printf("\t--shutdown\n");
    printf("\t--mt <number_of_mt_requests>\n");
    printf("\t-o output on\n");
    printf("\t--silent silent on\n");
    printf("\t--shm_prefix <prefix> (default: ZISK)\n");
    printf("\t-m metrics on\n");
    printf("\t-t trace on\n");
    printf("\t-tt trace_trace on\n");
    printf("\t-f(save to file)\n");
    printf("\t-a chunk_address\n");
    printf("\t-v verbose on\n");
    printf("\t-u unlock physical memory in mmap\n");
#ifdef ASM_PRECOMPILE_CACHE
    printf("\t--precompile-cache-store store precompile results in cache file\n");
    printf("\t--precompile-cache-load load precompile results from cache file\n");
#endif
    printf("\t-h/--help print this\n");
}

void parse_arguments(int argc, char *argv[])
{
    strcpy(shm_prefix, "ZISK");
    uint64_t number_of_selected_generation_methods = 0;
    if (argc > 1)
    {
        for (int i = 1; i < argc; i++)
        {
            if (strcmp(argv[i], "-s") == 0)
            {
                server = true;
                continue;
            }
            if (strcmp(argv[i], "-c") == 0)
            {
                client = true;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=0") == 0) || (strcmp(argv[i], "--generate_fast") == 0))
            {
                gen_method = Fast;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=1") == 0) || (strcmp(argv[i], "--generate_minimal_trace") == 0))
            {
                gen_method = MinimalTrace;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=2") == 0) || (strcmp(argv[i], "--generate_rom_histogram") == 0))
            {
                gen_method = RomHistogram;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=3") == 0) || (strcmp(argv[i], "--generate_main_trace") == 0))
            {
                gen_method = MainTrace;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=4") == 0) || (strcmp(argv[i], "--generate_chunks") == 0))
            {
                gen_method = ChunksOnly;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=6") == 0) || (strcmp(argv[i], "--generate_zip") == 0))
            {
                gen_method = Zip;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=7") == 0) || (strcmp(argv[i], "--generate_mem_op") == 0))
            {
                gen_method = MemOp;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=8") == 0) || (strcmp(argv[i], "--generate_chunk_player_mt_collect_mem") == 0))
            {
                gen_method = ChunkPlayerMTCollectMem;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=9") == 0) || (strcmp(argv[i], "--generate_mem_reads") == 0))
            {
                gen_method = MemReads;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=10") == 0) || (strcmp(argv[i], "--generate_chunk_player_mem_reads") == 0))
            {
                gen_method = ChunkPlayerMemReadsCollectMain;
                number_of_selected_generation_methods++;
                continue;
            }
            if (strcmp(argv[i], "-o") == 0)
            {
                output = true;
                continue;
            }
            if (strcmp(argv[i], "--silent") == 0)
            {
                silent = true;
                continue;
            }
            if (strcmp(argv[i], "-m") == 0)
            {
                metrics = true;
                continue;
            }
            if (strcmp(argv[i], "-t") == 0)
            {
                trace = true;
                continue;
            }
            if (strcmp(argv[i], "-tt") == 0)
            {
                trace = true;
                trace_trace = true;
                continue;
            }
            if (strcmp(argv[i], "-v") == 0)
            {
                verbose = true;
                //emu_verbose = true;
                continue;
            }
            if (strcmp(argv[i], "-u") == 0)
            {
                map_locked_flag = 0;
                continue;
            }
            if (strcmp(argv[i], "-h") == 0)
            {
                print_usage();
                exit(0);
            }
            if (strcmp(argv[i], "--help") == 0)
            {
                print_usage();
                continue;
            }
            if (strcmp(argv[i], "-i") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -i in the last position; please provide input file after it\n");
                    print_usage();
                    exit(-1);
                }
                if (strlen(argv[i]) > 4095)
                {
                    printf("ERROR: Detected argument -i but next argument is too long\n");
                    print_usage();
                    exit(-1);
                }
                strcpy(input_file, argv[i]);
                continue;
            }
            if (strcmp(argv[i], "--shm_prefix") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -i in the last position; please provide shared mem prefix after it\n");
                    print_usage();
                    exit(-1);
                }
                if (strlen(argv[i]) > MAX_SHM_PREFIX_LENGTH)
                {
                    printf("ERROR: Detected argument -i but next argument is too long\n");
                    print_usage();
                    exit(-1);
                }
                strcpy(shm_prefix, argv[i]);
                continue;
            }
            if (strcmp(argv[i], "--chunk") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -c in the last position; please provide chunk number after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                chunk_mask = strtoul(argv[i], &endptr, 10);

                // Check for errors
                if (errno == ERANGE) {
                    printf("ERROR: Chunk number is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argv[i]) {
                    printf("ERROR: No digits found while parsing chunk number\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("ERROR: Extra characters after chunk number: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else if (chunk_mask > MAX_CHUNK_MASK) {
                    printf("ERROR: Invalid chunk number: %lu\n", chunk_mask);
                    print_usage();
                    exit(-1);
                } else {
                    printf("Got chunk_mask= %lu\n", chunk_mask);
                }
                continue;
            }
            if (strcmp(argv[i], "--shutdown") == 0)
            {
                do_shutdown = true;
                continue;
            }
            if (strcmp(argv[i], "--mt") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -mt in the last position; please provide number of MT requests after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                number_of_mt_requests = strtoul(argv[i], &endptr, 10);

                // Check for errors
                if (errno == ERANGE) {
                    printf("ERROR: Number of MT requests is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argv[i]) {
                    printf("ERROR: No digits found while parsing number of MT requests\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("ERROR: Extra characters after number of MT requests: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else if (number_of_mt_requests > 1000000) {
                    printf("ERROR: Invalid number of MT requests: %lu\n", number_of_mt_requests);
                    print_usage();
                    exit(-1);
                } else {
                    printf("Got number of MT requests= %lu\n", number_of_mt_requests);
                }
                continue;
            }
            if (strcmp(argv[i], "-p") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -p in the last position; please provide port number after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                arguments_port = strtoul(argv[i], &endptr, 10);

                // Check for errors
                if (errno == ERANGE) {
                    printf("ERROR: Port number is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argv[i]) {
                    printf("ERROR: No digits found while parsing port number\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("ERROR: Extra characters after port number: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else {
                    printf("Got port number= %u\n", arguments_port);
                }
                continue;
            }
            if (strcmp(argv[i], "-f") == 0)
            {
                save_to_file = true;
                continue;
            }
            if (strcmp(argv[i], "-a") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("ERROR: Detected argument -a in the last position; please provide chunk address after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                char * argument = argv[i];
                if ((argument[0] == '0') && (argument[1] == 'x')) argument += 2;
                chunk_player_address = strtoul(argv[i], &endptr, 16);

                // Check for errors
                if (errno == ERANGE) {
                    printf("ERROR: Chunk address is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argument) {
                    printf("ERROR: No digits found while parsing chunk addresss\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("ERROR: Extra characters after chunk address: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else {
                    printf("Got chunk address= %p\n", (void *)chunk_player_address);
                }
                continue;
            }
#ifdef ASM_PRECOMPILE_CACHE
            if (strcmp(argv[i], "--precompile-cache-store") == 0)
            {
                precompile_cache_enabled = true;
                precompile_cache_store_init();
                continue;
            }
            if (strcmp(argv[i], "--precompile-cache-load") == 0)
            {
                precompile_cache_enabled = true;
                precompile_cache_load_init();
                continue;
            }

#endif
            printf("ERROR: parse_arguments() Unrecognized argument: %s\n", argv[i]);
            print_usage();
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    }
#ifdef ASM_PRECOMPILE_CACHE
    if (precompile_cache_enabled == false)
    {
        printf("ERROR: parse_arguments() when in precompile cache mode, you need to use an argument: either --precompile-cache-store or --precompile-cache-load\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
#endif

    // Check that only one generation method was selected as an argument
    if (number_of_selected_generation_methods != 1)
    {
        printf("ERROR! parse_arguments() Invalid arguments: select 1 generation method, and only one\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Check that the generation method selected by the process launcher is the same as the one
    // for which the assembly code was generated
    uint64_t asm_gen_method = get_gen_method();
    if (asm_gen_method != gen_method)
    {
        printf("ERROR! parse_arguments() Inconsistency: C generation method is %u but ASM generation method is %lu\n",
            gen_method,
            asm_gen_method);
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Check server/client
    if (server && client)
    {
        printf("ERROR! parse_arguments() Inconsistency: both server and client at the same time is not possible\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (!server && !client)
    {
        printf("ERROR! parse_arguments() Inconsistency: select server or client\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
}

void configure (void)
{
    // Select configuration based on generation method
    switch (gen_method)
    {
        case Fast:
        {
            strcpy(shmem_input_name, shm_prefix);
            strcat(shmem_input_name, "_FT_input");
            strcpy(shmem_output_name, "");
            strcpy(sem_chunk_done_name, "");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_FT_shutdown_done");
            strcpy(shmem_mt_name, "");
            port = 23120;
            break;
        }
        case MinimalTrace:
        {
            strcpy(shmem_input_name, shm_prefix);
            strcat(shmem_input_name, "_MT_input");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_MT_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_MT_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_MT_shutdown_done");
            strcpy(shmem_mt_name, "");
            call_chunk_done = true;
            port = 23115;
            break;
        }
        case RomHistogram:
        {
            strcpy(shmem_input_name, shm_prefix);
            strcat(shmem_input_name, "_RH_input");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_RH_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_RH_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_RH_shutdown_done");
            strcpy(shmem_mt_name, "");
            call_chunk_done = true;
            port = 23116;
            break;
        }
        case MainTrace:
        {
            strcpy(shmem_input_name, shm_prefix);
            strcat(shmem_input_name, "_MA_input");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_MA_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_MA_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_MA_shutdown_done");
            strcpy(shmem_mt_name, "");
            call_chunk_done = true;
            port = 23118;
            break;
        }
        case ChunksOnly:
        {
            strcpy(shmem_input_name, shm_prefix);
            strcat(shmem_input_name, "_CH_input");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_CH_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_CH_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_CH_shutdown_done");
            strcpy(shmem_mt_name, "");
            call_chunk_done = true;
            port = 23115;
            break;
        }
        // case BusOp:
        // {
        //     strcpy(shmem_input_name, "ZISKBO_input");
        //     strcpy(shmem_output_name, "ZISKBO_output");
        //     strcpy(sem_chunk_done_name, "ZISKBO_chunk_done");
        //     chunk_done = true;
        //     port = 23115;
        //     break;
        // }
        case Zip:
        {
            strcpy(shmem_input_name, shm_prefix);
            strcat(shmem_input_name, "_ZP_input");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_ZP_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_ZP_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_ZP_shutdown_done");
            strcpy(shmem_mt_name, "");
            call_chunk_done = true;
            port = 23115;
            break;
        }
        case MemOp:
        {
            strcpy(shmem_input_name, shm_prefix);
            strcat(shmem_input_name, "_MO_input");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_MO_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_MO_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_MO_shutdown_done");
            strcpy(shmem_mt_name, "");
            call_chunk_done = true;
            port = 23117;
            break;
        }
        case ChunkPlayerMTCollectMem:
        {
            strcpy(shmem_input_name, "");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_CM_output");
            strcpy(sem_chunk_done_name, "");
            strcpy(sem_shutdown_done_name, "");
            strcpy(shmem_mt_name, shm_prefix);
            strcat(shmem_mt_name, "_MT_output");
            call_chunk_done = false;
            port = 23119;
            break;
        }
        case MemReads:
        {
            strcpy(shmem_input_name, shm_prefix);
            strcat(shmem_input_name, "_MT_input");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_MT_output");
            strcpy(sem_chunk_done_name, shm_prefix);
            strcat(sem_chunk_done_name, "_MT_chunk_done");
            strcpy(sem_shutdown_done_name, shm_prefix);
            strcat(sem_shutdown_done_name, "_MT_shutdown_done");
            strcpy(shmem_mt_name, "");
            call_chunk_done = true;
            port = 23115;
            break;
        }
        case ChunkPlayerMemReadsCollectMain:
        {
            strcpy(shmem_input_name, "");
            strcpy(shmem_output_name, shm_prefix);
            strcat(shmem_output_name, "_CA_output");
            strcpy(sem_chunk_done_name, "");
            strcpy(sem_shutdown_done_name, "");
            strcpy(shmem_mt_name, shm_prefix);
            strcat(shmem_mt_name, "_MT_output");
            call_chunk_done = false;
            port = 23120;
            break;
        }
        default:
        {
            printf("ERROR: configure() Invalid gen_method = %u\n", gen_method);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    }

    if (arguments_port != 0)
    {
        port = arguments_port;
    }

    if (verbose)
    {
        printf("ziskemuasm configuration:\n");
        printf("\tgen_method=%u\n", gen_method);
        printf("\tshm_prefix=%s\n", shm_prefix);
        printf("\tport=%u\n", port);
        printf("\tcall_chunk_done=%u\n", call_chunk_done);
        printf("\tchunk_size=%lu\n", chunk_size);
        printf("\tshmem_input=%s\n", shmem_input_name);
        printf("\tshmem_output=%s\n", shmem_output_name);
        printf("\tshmem_mt=%s\n", shmem_mt_name);
        printf("\tsem_chunk_done=%s\n", sem_chunk_done_name);
        printf("\tsem_shutdown_done=%s\n", sem_shutdown_done_name);
        printf("\tmap_locked_flag=%d\n", map_locked_flag);
        printf("\toutput=%u\n", output);
    }
}

void client_setup (void)
{
    assert(!server);
    assert(client);

    int result;

    /***********************/
    /* INPUT MINIMAL TRACE */
    /***********************/

    // Input MT trace
    if ((gen_method == ChunkPlayerMTCollectMem) || (gen_method == ChunkPlayerMemReadsCollectMain))
    {
        // Create the output shared memory
        shmem_mt_fd = shm_open(shmem_mt_name, O_RDONLY, 0666);
        if (shmem_mt_fd < 0)
        {
            printf("ERROR: Failed calling shm_open(%s) errno=%d=%s\n", shmem_mt_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map it to the trace address
#ifdef DEBUG
        gettimeofday(&start_time, NULL);
#endif
        void * pTrace = mmap((void *)TRACE_ADDR, chunk_player_mt_size, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_mt_fd, 0);
#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
#endif
        if (pTrace == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(MT) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pTrace != TRACE_ADDR)
        {
            printf("ERROR: Called mmap(MT) but returned address = %p != 0x%lx\n", pTrace, TRACE_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("mmap(MT) returned %p in %lu us\n", pTrace, duration);
    }
}

void client_run (void)
{
    assert(client);
    assert(!server);

    int result;

    /************************/
    /* Read input file data */
    /************************/
    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {

#ifdef DEBUG
        gettimeofday(&start_time, NULL);
#endif

        // Open input file
        FILE * input_fp = fopen(input_file, "r");
        if (input_fp == NULL)
        {
            printf("ERROR: Failed calling fopen(%s) errno=%d=%s; does it exist?\n", input_file, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Get input file size
        if (fseek(input_fp, 0, SEEK_END) == -1)
        {
            printf("ERROR: Failed calling fseek(%s) errno=%d=%s\n", input_file, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        long input_data_size = ftell(input_fp);
        if (input_data_size == -1)
        {
            printf("ERROR: Failed calling ftell(%s) errno=%d=%s\n", input_file, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Go back to the first byte
        if (fseek(input_fp, 0, SEEK_SET) == -1)
        {
            printf("ERROR: Failed calling fseek(%s, 0) errno=%d=%s\n", input_file, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Check the input data size is inside the proper range
        if (input_data_size > (MAX_INPUT_SIZE - 16))
        {
            printf("ERROR: Size of input file (%s) is too long (%lu)\n", input_file, input_data_size);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Open input shared memory
        shmem_input_fd = shm_open(shmem_input_name, O_RDWR, 0666);
        if (shmem_input_fd < 0)
        {
            printf("ERROR: Failed calling shm_open(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map the shared memory object into the process address space
        shmem_input_address = mmap(NULL, MAX_INPUT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, shmem_input_fd, 0);
        if (shmem_input_address == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Write the input size in the first 64 bits
        *(uint64_t *)shmem_input_address = (uint64_t)0; // free input
        *(uint64_t *)(shmem_input_address + 8)= (uint64_t)input_data_size;

        // Copy input data into input memory
        size_t input_read = fread(shmem_input_address + 16, 1, input_data_size, input_fp);
        if (input_read != input_data_size)
        {
            printf("ERROR: Input read (%lu) != input file size (%lu)\n", input_read, input_data_size);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Close the file pointer
        fclose(input_fp);

        // Unmap input
        result = munmap(shmem_input_address, MAX_INPUT_SIZE);
        if (result == -1)
        {
            printf("ERROR: Failed calling munmap(input) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        printf("client (input): done in %lu us\n", duration);
#endif

    }

    /*************************/
    /* Connect to the server */
    /*************************/
    
    // Create socket to connect to server
    int socket_fd;
    socket_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (socket_fd < 0)
    {
        printf("ERROR: socket() failed socket_fd=%d errno=%d=%s\n", socket_fd, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Configure server address
    struct sockaddr_in server_addr;
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(port);
    
    result = inet_pton(AF_INET, SERVER_IP, &server_addr.sin_addr);
    if (result <= 0)
    {
        printf("ERROR: inet_pton() failed.  Invalid address/Address not supported result=%d errno=%d=%s\n", result, errno, strerror(errno));
        exit(-1);
    }

    // Connect to server
    result = connect(socket_fd, (struct sockaddr *)&server_addr, sizeof(server_addr));
    if (result < 0)
    {
        printf("ERROR: connect() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        exit(-1);
    }
    if (verbose) printf("connect()'d to port=%u\n", port);

    // Request and response
    uint64_t request[5];
    uint64_t response[5];

    /********/
    /* Ping */
    /********/

    gettimeofday(&start_time, NULL);

    // Prepare message to send
    request[0] = TYPE_PING;
    request[1] = 0;
    request[2] = 0;
    request[3] = 0;
    request[4] = 0;

    // Send data to server
    result = send(socket_fd, request, sizeof(request), 0);
    if (result < 0)
    {
        printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Read server response
    ssize_t bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
    if (bytes_received < 0)
    {
        printf("ERROR: recv_all_with_timeout() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (bytes_received != sizeof(response))
    {
        printf("ERROR: recv_all_with_timeout() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (response[0] != TYPE_PONG)
    {
        printf("ERROR: recv_all_with_timeout() returned unexpected type=%lu\n", response[0]);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (response[1] != gen_method)
    {
        printf("ERROR: recv_all_with_timeout() returned unexpected gen_method=%lu\n", response[1]);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    printf("client (PING): done in %lu us\n", duration);

    /*****************/
    /* Minimal trace */
    /*****************/
    for (uint64_t i=0; i<number_of_mt_requests; i++)
    {
        switch (gen_method)
        {
            case MinimalTrace:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_MT_REQUEST;
                request[1] = 1ULL << 32; // max_steps
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                result = send(socket_fd, request, sizeof(request), 0);
                if (result < 0)
                {
                    printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }

                // Read server response
                bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                if (bytes_received < 0)
                {
                    printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (bytes_received != sizeof(response))
                {
                    printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[0] != TYPE_MT_RESPONSE)
                {
                    printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                printf("client (MT)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));

                break;
            }
            case RomHistogram:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_RH_REQUEST;
                request[1] = 1ULL << 32; // max_steps
                request[2] = 0;
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                result = send(socket_fd, request, sizeof(request), 0);
                if (result < 0)
                {
                    printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }

                // Read server response
                bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                if (bytes_received < 0)
                {
                    printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (bytes_received != sizeof(response))
                {
                    printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[0] != TYPE_RH_RESPONSE)
                {
                    printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                printf("client (RH)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));

                break;
            }
            case MemOp:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_MO_REQUEST;
                request[1] = 1ULL << 32; // max_steps
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                result = send(socket_fd, request, sizeof(request), 0);
                if (result < 0)
                {
                    printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }

                // Read server response
                bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                if (bytes_received < 0)
                {
                    printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (bytes_received != sizeof(response))
                {
                    printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[0] != TYPE_MO_RESPONSE)
                {
                    printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                printf("client (MO)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));
                
                break;
            }
            case MainTrace:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_MA_REQUEST;
                request[1] = 1ULL << 32; // max_steps
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                result = send(socket_fd, request, sizeof(request), 0);
                if (result < 0)
                {
                    printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }

                // Read server response
                bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                if (bytes_received < 0)
                {
                    printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (bytes_received != sizeof(response))
                {
                    printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[0] != TYPE_MA_RESPONSE)
                {
                    printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                printf("client (MA)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));
                
                break;
            }
            case ChunkPlayerMTCollectMem:
            {
                if (chunk_player_address != 0)
                {
                    gettimeofday(&start_time, NULL);

                    // Prepare message to send
                    request[0] = TYPE_CM_REQUEST;
                    request[1] = 1ULL << 32; // max_steps
                    request[2] = 1ULL << 18; // chunk_len
                    request[3] = chunk_player_address;
                    request[4] = 0;

                    // Send data to server
                    result = send(socket_fd, request, sizeof(request), 0);
                    if (result < 0)
                    {
                        printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }

                    // Read server response
                    bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                    if (bytes_received < 0)
                    {
                        printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }
                    if (bytes_received != sizeof(response))
                    {
                        printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }
                    if (response[0] != TYPE_CM_RESPONSE)
                    {
                        printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }
                    if (response[1] != 0)
                    {
                        printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }
                    
                    gettimeofday(&stop_time, NULL);
                    duration = TimeDiff(start_time, stop_time);
                    printf("client (CM)[%lu]: done in %lu us\n", i, duration);
                }
                else
                {
                    uint64_t number_of_chunks = pInputTrace[4];
                    printf("client (CM)[%lu]: sending requests for %lu chunks\n", i, number_of_chunks);

                    for (uint64_t c = 0; c < number_of_chunks; c++)
                    {
                        if (c == 0)
                        {
                            chunk_player_address = 0xc0000028;
                        }
                        else
                        {
                            uint64_t * chunk = (uint64_t *)chunk_player_address;
                            uint64_t mem_reads_size = chunk[40];
                            chunk_player_address += (41 + mem_reads_size) * 8;
                        }

                        printf("client (CM)[%lu][%lu]: @=0x%lx sending request...", i, c, chunk_player_address);

                        gettimeofday(&start_time, NULL);
    
                        // Prepare message to send
                        request[0] = TYPE_CM_REQUEST;
                        request[1] = 1ULL << 32; // max_steps
                        request[2] = 1ULL << 18; // chunk_len
                        request[3] = chunk_player_address;
                        request[4] = 0;
    
                        // Send data to server
                        result = send(socket_fd, request, sizeof(request), 0);
                        if (result < 0)
                        {
                            printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
    
                        // Read server response
                        bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                        if (bytes_received < 0)
                        {
                            printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                        if (bytes_received != sizeof(response))
                        {
                            printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                        if (response[0] != TYPE_CM_RESPONSE)
                        {
                            printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                        if (response[1] != 0)
                        {
                            printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                        
                        gettimeofday(&stop_time, NULL);
                        duration = TimeDiff(start_time, stop_time);
                        printf("done in %lu us\n", duration);
                    }

                } 
                
                break;
            }
            case Fast:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_FA_REQUEST;
                request[1] = 1ULL << 32; // max_steps
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                result = send(socket_fd, request, sizeof(request), 0);
                if (result < 0)
                {
                    printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }

                // Read server response
                bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                if (bytes_received < 0)
                {
                    printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (bytes_received != sizeof(response))
                {
                    printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[0] != TYPE_FA_RESPONSE)
                {
                    printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                printf("client (FA)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));
                
                break;
            }
            case MemReads:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_MR_REQUEST;
                request[1] = 1ULL << 32; // max_steps
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                result = send(socket_fd, request, sizeof(request), 0);
                if (result < 0)
                {
                    printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }

                // Read server response
                bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                if (bytes_received < 0)
                {
                    printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (bytes_received != sizeof(response))
                {
                    printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[0] != TYPE_MR_RESPONSE)
                {
                    printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                printf("client (MR)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));

                break;
            }
            case ChunkPlayerMemReadsCollectMain:
            {
                if (chunk_player_address != 0)
                {
                    gettimeofday(&start_time, NULL);

                    // Prepare message to send
                    request[0] = TYPE_CA_REQUEST;
                    request[1] = 1ULL << 32; // max_steps
                    request[2] = 1ULL << 18; // chunk_len
                    request[3] = chunk_player_address;
                    request[4] = 0;

                    // Send data to server
                    result = send(socket_fd, request, sizeof(request), 0);
                    if (result < 0)
                    {
                        printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }

                    // Read server response
                    bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                    if (bytes_received < 0)
                    {
                        printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }
                    if (bytes_received != sizeof(response))
                    {
                        printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }
                    if (response[0] != TYPE_CA_RESPONSE)
                    {
                        printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }
                    if (response[1] != 0)
                    {
                        printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }
                    
                    gettimeofday(&stop_time, NULL);
                    duration = TimeDiff(start_time, stop_time);
                    printf("client (CA)[%lu]: done in %lu us\n", i, duration);
                }
                else
                {
                    uint64_t number_of_chunks = pInputTrace[4];
                    printf("client (CA)[%lu]: sending requests for %lu chunks\n", i, number_of_chunks);

                    for (uint64_t c = 0; c < number_of_chunks; c++)
                    {
                        if (c == 0)
                        {
                            chunk_player_address = 0xc0000028;
                        }
                        else
                        {
                            uint64_t * chunk = (uint64_t *)chunk_player_address;
                            uint64_t mem_reads_size = chunk[40];
                            chunk_player_address += (41 + mem_reads_size) * 8;
                        }

                        printf("client (CA)[%lu][%lu]: @=0x%lx sending request...", i, c, chunk_player_address);

                        gettimeofday(&start_time, NULL);
    
                        // Prepare message to send
                        request[0] = TYPE_CA_REQUEST;
                        request[1] = 1ULL << 32; // max_steps
                        request[2] = 1ULL << 18; // chunk_len
                        request[3] = chunk_player_address;
                        request[4] = 0;
    
                        // Send data to server
                        result = send(socket_fd, request, sizeof(request), 0);
                        if (result < 0)
                        {
                            printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
    
                        // Read server response
                        bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
                        if (bytes_received < 0)
                        {
                            printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                        if (bytes_received != sizeof(response))
                        {
                            printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                        if (response[0] != TYPE_CA_RESPONSE)
                        {
                            printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                        if (response[1] != 0)
                        {
                            printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                        
                        gettimeofday(&stop_time, NULL);
                        duration = TimeDiff(start_time, stop_time);
                        printf("done in %lu us\n", duration);
                    }

                } 
                
                break;
            }
            default:
            {
                printf("client_run() found invalid gen_method=%d\n", gen_method);
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }
        }
    } // number_of_mt_requests

    /************/
    /* Shutdown */
    /************/

    if (do_shutdown)
    {

    gettimeofday(&start_time, NULL);

    // Prepare message to send
    request[0] = TYPE_SD_REQUEST;
    request[1] = 0;
    request[2] = 0;
    request[3] = 0;
    request[4] = 0;

    // Send data to server
    result = send(socket_fd, request, sizeof(request), 0);
    if (result < 0)
    {
        printf("ERROR: send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Read server response
    bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
    if (bytes_received < 0)
    {
        printf("ERROR: recv() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (bytes_received != sizeof(response))
    {
        printf("ERROR: recv() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (response[0] != TYPE_SD_RESPONSE)
    {
        printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    printf("client (SD): done in %lu us\n", duration);

    } // do_shutdown

    /***********/
    /* Cleanup */
    /***********/

    // Close the socket
    close(socket_fd);
}

void client_cleanup (void)
{
    // Cleanup trace
    int result = munmap((void *)TRACE_ADDR, trace_size);
    if (result == -1)
    {
        printf("ERROR: Failed calling munmap(trace) for size=%lu errno=%d=%s\n", trace_size, errno, strerror(errno));
    }
}

void server_setup (void)
{
    assert(server);
    assert(!client);

    int result;

    /*******/
    /* ROM */
    /*******/
    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {

        if (verbose) gettimeofday(&start_time, NULL);
        void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED | map_locked_flag, -1, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pRom == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(rom) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pRom != ROM_ADDR)
        {
            printf("ERROR: Called mmap(rom) but returned address = %p != 0x%lx\n", pRom, ROM_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("mmap(rom) mapped %ld B and returned address %p in %lu us\n", ROM_SIZE, pRom, duration);
    }

    /*********/
    /* INPUT */
    /*********/

    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
        // Make sure the input shared memory is deleted
        shm_unlink(shmem_input_name);

        // Create the input shared memory
        shmem_input_fd = shm_open(shmem_input_name, O_RDWR | O_CREAT, 0666);
        if (shmem_input_fd < 0)
        {
            printf("ERROR: Failed calling shm_open(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Size it
        result = ftruncate(shmem_input_fd, MAX_INPUT_SIZE);
        if (result != 0)
        {
            printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map input address space
        if (verbose) gettimeofday(&start_time, NULL);
        void * pInput = mmap((void *)INPUT_ADDR, MAX_INPUT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_input_fd, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pInput == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(input) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pInput != INPUT_ADDR)
        {
            printf("ERROR: Called mmap(pInput) but returned address = %p != 0x%lx\n", pInput, INPUT_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("mmap(input) mapped %lu B and returned address %p in %lu us\n", MAX_INPUT_SIZE, pInput, duration);
    }

    /*******/
    /* RAM */
    /*******/

    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {

        if (verbose) gettimeofday(&start_time, NULL);
        void * pRam = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED | map_locked_flag, -1, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pRam == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(ram) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pRam != RAM_ADDR)
        {
            printf("ERROR: Called mmap(ram) but returned address = %p != 0x%08lx\n", pRam, RAM_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("mmap(ram) mapped %lu B and returned address %p in %lu us\n", RAM_SIZE, pRam, duration);
    }

    /****************/
    /* OUTPUT TRACE */
    /****************/

    // If ROM histogram, configure trace size
    if (gen_method == RomHistogram)
    {
        // Get max PC values for low and high addresses
        uint64_t max_bios_pc = get_max_bios_pc();
        uint64_t max_program_pc = get_max_program_pc();
        assert(max_bios_pc >= 0x1000);
        assert((max_bios_pc & 0x3) == 0);
        assert(max_program_pc >= 0x80000000);

        // Calculate sizes
        bios_size = ((max_bios_pc - 0x1000) >> 2) + 1;
        program_size = max_program_pc - 0x80000000 + 1;
        histogram_size = (4 + 1 + bios_size + 1 + program_size)*8;
#define TRACE_SIZE_GRANULARITY (1014*1014)
        initial_trace_size = ((histogram_size/TRACE_SIZE_GRANULARITY) + 1) * TRACE_SIZE_GRANULARITY;
        trace_size = initial_trace_size;
    }

    // Output trace
    if ((gen_method == MinimalTrace) ||
        (gen_method == RomHistogram) ||
        (gen_method == MainTrace) ||
        (gen_method == Zip) ||
        (gen_method == MemOp) ||
        (gen_method == ChunkPlayerMTCollectMem) ||
        (gen_method == MemReads) ||
        (gen_method == ChunkPlayerMemReadsCollectMain))
    {
        // Make sure the output shared memory is deleted
        shm_unlink(shmem_output_name);

        // Create the output shared memory
        shmem_output_fd = shm_open(shmem_output_name, O_RDWR | O_CREAT, 0666);
        if (shmem_output_fd < 0)
        {
            printf("ERROR: Failed calling shm_open(%s) errno=%d=%s\n", shmem_output_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Size it
        result = ftruncate(shmem_output_fd, trace_size);
        if (result != 0)
        {
            printf("ERROR: Failed calling ftruncate(%s) errno=%d=%s\n", shmem_output_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map it to the trace address
        if (verbose) gettimeofday(&start_time, NULL);
        void * requested_address;
        if ((gen_method == ChunkPlayerMTCollectMem) || (gen_method == ChunkPlayerMemReadsCollectMain))
        {
            requested_address = 0;
        }
        else
        {
            requested_address = (void *)TRACE_ADDR;
        }
        int flags = MAP_SHARED | map_locked_flag;
        if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
        {
            flags |= MAP_FIXED;
        }
        void * pTrace = mmap(requested_address, trace_size, PROT_READ | PROT_WRITE, flags, shmem_output_fd, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pTrace == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(pTrace) name=%s errno=%d=%s\n", shmem_output_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain) && ((uint64_t)pTrace != TRACE_ADDR))
        {
            printf("ERROR: Called mmap(trace) but returned address = %p != 0x%lx\n", pTrace, TRACE_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("mmap(trace) mapped %lu B and returned address %p in %lu us\n", trace_size, pTrace, duration);

        trace_address = (uint64_t)pTrace;
        pOutputTrace = pTrace;
    }

    /***********************/
    /* INPUT MINIMAL TRACE */
    /***********************/

    // Input MT trace
    if ((gen_method == ChunkPlayerMTCollectMem) || (gen_method == ChunkPlayerMemReadsCollectMain))
    {
        // Create the output shared memory
        shmem_mt_fd = shm_open(shmem_mt_name, O_RDONLY, 0666);
        if (shmem_mt_fd < 0)
        {
            printf("ERROR: Failed calling shm_open(%s) errno=%d=%s\n", shmem_mt_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map it to the trace address
#ifdef DEBUG
        gettimeofday(&start_time, NULL);
#endif
        void * pTrace = mmap((void *)TRACE_ADDR, chunk_player_mt_size, PROT_READ, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_mt_fd, 0);
#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
#endif
        if (pTrace == MAP_FAILED)
        {
            printf("ERROR: Failed calling mmap(MT) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pTrace != TRACE_ADDR)
        {
            printf("ERROR: Called mmap(MT) but returned address = %p != 0x%lx\n", pTrace, TRACE_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("mmap(MT) returned %p in %lu us\n", pTrace, duration);
    }

    /******************/
    /* SEM CHUNK DONE */
    /******************/

    if (call_chunk_done)
    {
        assert(strlen(sem_chunk_done_name) > 0);

        sem_unlink(sem_chunk_done_name);

        sem_chunk_done = sem_open(sem_chunk_done_name, O_CREAT, 0666, 0);
        if (sem_chunk_done == SEM_FAILED)
        {
            printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("sem_open(%s) succeeded\n", sem_chunk_done_name);
    }

    /*********************/
    /* SEM SHUTDOWN DONE */
    /*********************/
    
    assert(strlen(sem_shutdown_done_name) > 0);

    sem_unlink(sem_shutdown_done_name);
    
    sem_shutdown_done = sem_open(sem_shutdown_done_name, O_CREAT, 0666, 0);
    if (sem_shutdown_done == SEM_FAILED)
    {
        printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_shutdown_done_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (verbose) printf("sem_open(%s) succeeded\n", sem_shutdown_done_name);

    /* Write read-only ROM data */
    write_ro_data();
}

void server_reset (void)
{
    // Reset RAM data for next emulation
    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain))
    {
#ifdef DEBUG
        gettimeofday(&start_time, NULL);
#endif
        memset((void *)RAM_ADDR, 0, RAM_SIZE);
#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        if (verbose) printf("server_reset() memset(ram) in %lu us\n", duration);
#endif
        if ((gen_method != Fast) && (gen_method != RomHistogram))
        {
            // Reset trace: init output header data
            pOutputTrace[0] = 0x000100; // Version, e.g. v1.0.0 [8]
            pOutputTrace[1] = 1; // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
            pOutputTrace[2] = trace_size; // MT allocated size [8] -> to be updated after reallocation
            pOutputTrace[3] = 0; // MT used size [8] -> to be updated after completion
            
            // Reset trace used size
            trace_used_size = 0;
        }
    }
}

void server_run (void)
{
    if ((gen_method == RomHistogram)) {
        memset((void *)trace_address, 0, trace_size);
    }

#ifdef ASM_CALL_METRICS
    reset_asm_call_metrics();
#endif

    // Init trace header
    if ((gen_method != ChunkPlayerMTCollectMem) && (gen_method != ChunkPlayerMemReadsCollectMain) && (gen_method != Fast))
    {
        // Reset trace: init output header data
        pOutputTrace[0] = 0x000100; // Version, e.g. v1.0.0 [8]
        pOutputTrace[1] = 1; // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
        pOutputTrace[2] = trace_size; // MT allocated size [8] -> to be updated after reallocation
        pOutputTrace[3] = 0; // MT used size [8] -> to be updated after completion
        
        // Reset trace used size
        trace_used_size = 0;
    }

    /*******/
    /* ASM */
    /*******/

    // Call emulator assembly code
    gettimeofday(&start_time,NULL);
    if (verbose) printf("trace_address=%lx\n", trace_address);
    emulator_start();
    gettimeofday(&stop_time,NULL);
    assembly_duration = TimeDiff(start_time, stop_time);

    uint64_t final_trace_size = MEM_CHUNK_ADDRESS - MEM_TRACE_ADDRESS;
    trace_used_size = final_trace_size + 32;

    if ( metrics )
    {
        uint64_t duration = assembly_duration;
        uint64_t steps = MEM_STEP;
        uint64_t end = MEM_END;
        uint64_t error = MEM_ERROR;
        uint64_t step_duration_ns = steps == 0 ? 0 : (duration * 1000) / steps;
        uint64_t step_tp_sec = duration == 0 ? 0 : steps * 1000000 / duration;
        uint64_t final_trace_size_percentage = (final_trace_size * 100) / trace_size;
        printf("Duration = %lu us, realloc counter = %lu, steps = %lu, step duration = %lu ns, tp = %lu steps/s, trace size = 0x%lx - 0x%lx = %lu B(%lu%%), end=%lu, error=%lu, max steps=%lu, chunk size=%lu\n",
            duration,
            realloc_counter,
            steps,
            step_duration_ns,
            step_tp_sec,
            MEM_CHUNK_ADDRESS,
            MEM_TRACE_ADDRESS,
            final_trace_size,
            final_trace_size_percentage,
            end,
            error,
            max_steps,
            chunk_size);
        if (gen_method == RomHistogram)
        {
            printf("Rom histogram size=%lu\n", histogram_size);
        }
    }
    if (MEM_ERROR)
    {
        printf("Emulation ended with error code %lu\n", MEM_ERROR);
    }

    // Log output
    if (output)
    {
        unsigned int * pOutput = (unsigned int *)OUTPUT_ADDR;
        unsigned int output_size = *pOutput;
#ifdef DEBUG
        if (verbose) printf("Output size=%d\n", output_size);
#endif

        for (unsigned int i = 0; i < output_size; i++)
        {
            pOutput++;
            printf("%08x\n", *pOutput);
        }
    }

    // Complete output header data
    if ((gen_method == MinimalTrace) ||
        (gen_method == RomHistogram) ||
        (gen_method == Zip) ||
        (gen_method == MainTrace) ||
        (gen_method == MemOp) ||
        (gen_method == MemReads) ||
        (gen_method == ChunkPlayerMemReadsCollectMain))
    {
        uint64_t * pOutput = (uint64_t *)trace_address;
        pOutput[0] = 0x000100; // Version, e.g. v1.0.0 [8]
        pOutput[1] = MEM_ERROR; // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
        pOutput[2] = trace_size; // MT allocated size [8]
        //assert(final_trace_size > 32);
        if (gen_method == RomHistogram)
        {
            pOutput[3] = MEM_STEP;
            pOutput[4] = bios_size;
            pOutput[4 + bios_size + 1] = program_size;
        }
        else
        {
            pOutput[3] = trace_used_size; // MT used size [8]
        }
    }

    // Notify client
    if (gen_method == RomHistogram)
    {
        _chunk_done();   
    }


    // Notify the caller that the trace is ready to be consumed
    // if (!is_file)
    // {
    //     result = sem_post(sem_input);
    //     if (result == -1)
    //     {
    //         printf("Failed calling sem_post(%s) errno=%d=%s\n", sem_input_name, errno, strerror(errno));
    //         fflush(stdout);
    //         fflush(stderr);
    //         exit(-1);
    //     }
    // }


#ifdef ASM_CALL_METRICS
    print_asm_call_metrics(assembly_duration);
#endif

    // Log trace
    if (((gen_method == MinimalTrace) || (gen_method == Zip)) && trace)
    {
        log_minimal_trace();
    }
    if ((gen_method == RomHistogram) && trace)
    {
        log_histogram();
    }
    if ((gen_method == MainTrace) && trace)
    {
        log_main_trace();
    }
    if ((gen_method == MemOp) && trace)
    {
        log_mem_op();
    }
    if ((gen_method == MemOp) && save_to_file)
    {
        save_mem_op_to_files();
    }
    if ((gen_method == ChunkPlayerMTCollectMem) && trace)
    {
        log_mem_trace();
    }
    if ((gen_method == MemReads) && trace)
    {
        log_minimal_trace();
    }
    if ((gen_method == ChunkPlayerMemReadsCollectMain) && trace)
    {
        log_chunk_player_main_trace();
    }
}

void server_cleanup (void)
{
    // Cleanup ROM
    int result = munmap((void *)ROM_ADDR, ROM_SIZE);
    if (result == -1)
    {
        printf("ERROR: Failed calling munmap(rom) errno=%d=%s\n", errno, strerror(errno));
    }

    // Cleanup RAM
    result = munmap((void *)RAM_ADDR, RAM_SIZE);
    if (result == -1)
    {
        printf("ERROR: Failed calling munmap(ram) errno=%d=%s\n", errno, strerror(errno));
    }

    // Cleanup INPUT
    result = munmap((void *)INPUT_ADDR, MAX_INPUT_SIZE);
    if (result == -1)
    {
        printf("ERROR: Failed calling munmap(input) errno=%d=%s\n", errno, strerror(errno));
    }
    result = shm_unlink(shmem_input_name);
    if (result == -1)
    {
        printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
    }

    // Cleanup trace
    result = munmap((void *)TRACE_ADDR, trace_size);
    if (result == -1)
    {
        printf("ERROR: Failed calling munmap(trace) for size=%lu errno=%d=%s\n", trace_size, errno, strerror(errno));
    }
    result = shm_unlink(shmem_output_name);
    if (result == -1)
    {
        printf("ERROR: Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_output_name, errno, strerror(errno));
    }

    // Cleanup chunk done semaphore
    if (call_chunk_done)
    {
        result = sem_close(sem_chunk_done);
        if (result == -1)
        {
            printf("ERROR: Failed calling sem_close(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_chunk_done_name);
        if (result == -1)
        {
            printf("ERROR: Failed calling sem_unlink(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        }
    }

    // Post shutdown donw semaphore
    result = sem_post(sem_shutdown_done);
    if (result == -1)
    {
        printf("ERROR: Failed calling sem_post(%s) errno=%d=%s\n", sem_shutdown_done_name, errno, strerror(errno));
    }
}

// extern uint64_t reg_0;
// extern uint64_t reg_1;
// extern uint64_t reg_2;
// extern uint64_t reg_3;
// extern uint64_t reg_4;
// extern uint64_t reg_5;
// extern uint64_t reg_6;
// extern uint64_t reg_7;
// extern uint64_t reg_8;
// extern uint64_t reg_9;
// extern uint64_t reg_10;
// extern uint64_t reg_11;
// extern uint64_t reg_12;
// extern uint64_t reg_13;
// extern uint64_t reg_14;
// extern uint64_t reg_15;
// extern uint64_t reg_16;
// extern uint64_t reg_17;
// extern uint64_t reg_18;
// extern uint64_t reg_19;
// extern uint64_t reg_20;
// extern uint64_t reg_21;
// extern uint64_t reg_22;
// extern uint64_t reg_23;
// extern uint64_t reg_24;
// extern uint64_t reg_25;
// extern uint64_t reg_26;
// extern uint64_t reg_27;
// extern uint64_t reg_28;
// extern uint64_t reg_29;
// extern uint64_t reg_30;
// extern uint64_t reg_31;
// extern uint64_t reg_32;
// extern uint64_t reg_33;
// extern uint64_t reg_34;

extern int _print_regs()
{
    // printf("print_regs()\n");
    // printf("\treg[ 0]=%lu=0x%lx=@%p\n", reg_0,  reg_0,  &reg_0);
    // //printf("\treg[ 1]=%lu=0x%lx=@%p\n", reg_1,  reg_1,  &reg_1);
    // //printf("\treg[ 2]=%lu=0x%lx=@%p\n", reg_2,  reg_2,  &reg_2);
    // printf("\treg[ 3]=%lu=0x%lx=@%p\n", reg_3,  reg_3,  &reg_3);
    // printf("\treg[ 4]=%lu=0x%lx=@%p\n", reg_4,  reg_4,  &reg_4);
    // /*printf("\treg[ 5]=%lu=0x%lx=@%p\n", reg_5,  reg_5,  &reg_5);
    // printf("\treg[ 6]=%lu=0x%lx=@%p\n", reg_6,  reg_6,  &reg_6);
    // printf("\treg[ 7]=%lu=0x%lx=@%p\n", reg_7,  reg_7,  &reg_7);
    // printf("\treg[ 8]=%lu=0x%lx=@%p\n", reg_8,  reg_8,  &reg_8);
    // printf("\treg[ 9]=%lu=0x%lx=@%p\n", reg_9,  reg_9,  &reg_9);
    // printf("\treg[10]=%lu=0x%lx=@%p\n", reg_10, reg_10, &reg_10);
    // printf("\treg[11]=%lu=0x%lx=@%p\n", reg_11, reg_11, &reg_11);
    // printf("\treg[12]=%lu=0x%lx=@%p\n", reg_12, reg_12, &reg_12);
    // printf("\treg[13]=%lu=0x%lx=@%p\n", reg_13, reg_13, &reg_13);
    // printf("\treg[14]=%lu=0x%lx=@%p\n", reg_14, reg_14, &reg_14);
    // printf("\treg[15]=%lu=0x%lx=@%p\n", reg_15, reg_15, &reg_15);
    // printf("\treg[16]=%lu=0x%lx=@%p\n", reg_16, reg_16, &reg_16);
    // printf("\treg[17]=%lu=0x%lx=@%p\n", reg_17, reg_17, &reg_17);
    // printf("\treg[18]=%lu=0x%lx=@%p\n", reg_18, reg_18, &reg_18);*/
    // printf("\treg[19]=%lu=0x%lx=@%p\n", reg_19, reg_19, &reg_19);
    // printf("\treg[20]=%lu=0x%lx=@%p\n", reg_20, reg_20, &reg_20);
    // printf("\treg[21]=%lu=0x%lx=@%p\n", reg_21, reg_21, &reg_21);
    // printf("\treg[22]=%lu=0x%lx=@%p\n", reg_22, reg_22, &reg_22);
    // printf("\treg[23]=%lu=0x%lx=@%p\n", reg_23, reg_23, &reg_23);
    // printf("\treg[24]=%lu=0x%lx=@%p\n", reg_24, reg_24, &reg_24);
    // printf("\treg[25]=%lu=0x%lx=@%p\n", reg_25, reg_25, &reg_25);
    // printf("\treg[26]=%lu=0x%lx=@%p\n", reg_26, reg_26, &reg_26);
    // printf("\treg[27]=%lu=0x%lx=@%p\n", reg_27, reg_27, &reg_27);
    // printf("\treg[28]=%lu=0x%lx=@%p\n", reg_28, reg_28, &reg_28);
    // printf("\treg[29]=%lu=0x%lx=@%p\n", reg_29, reg_29, &reg_29);
    // printf("\treg[30]=%lu=0x%lx=@%p\n", reg_30, reg_30, &reg_30);
    // printf("\treg[31]=%lu=0x%lx=@%p\n", reg_31, reg_31, &reg_31);
    // printf("\treg[32]=%lu=0x%lx=@%p\n", reg_32, reg_32, &reg_32);
    // printf("\treg[33]=%lu=0x%lx=@%p\n", reg_33, reg_33, &reg_33);
    // printf("\treg[34]=%lu=0x%lx=@%p\n", reg_34, reg_34, &reg_34);
    // printf("\n");
}

extern int _print_pc (uint64_t pc, uint64_t c)
{
    printf("s=%lu pc=%lx c=%lx", print_pc_counter, pc, c);
    /* Used for debugging
    printf(" r0=%lx", reg_0);
    printf(" r1=%lx", reg_1);
    printf(" r2=%lx", reg_2);
    printf(" r3=%lx", reg_3);
    printf(" r4=%lx", reg_4);
    printf(" r5=%lx", reg_5);
    printf(" r6=%lx", reg_6);
    printf(" r7=%lx", reg_7);
    printf(" r8=%lx", reg_8);
    printf(" r9=%lx", reg_9);
    printf(" r10=%lx", reg_10);
    printf(" r11=%lx", reg_11);
    printf(" r12=%lx", reg_12);
    printf(" r13=%lx", reg_13);
    printf(" r14=%lx", reg_14);
    printf(" r15=%lx", reg_15);
    printf(" r16=%lx", reg_16);
    printf(" r17=%lx", reg_17);
    printf(" r18=%lx", reg_18);
    printf(" r19=%lx", reg_19);
    printf(" r20=%lx", reg_20);
    printf(" r21=%lx", reg_21);
    printf(" r22=%lx", reg_22);
    printf(" r23=%lx", reg_23);
    printf(" r24=%lx", reg_24);
    printf(" r25=%lx", reg_25);
    printf(" r26=%lx", reg_26);
    printf(" r27=%lx", reg_27);
    printf(" r28=%lx", reg_28);
    printf(" r29=%lx", reg_29);
    printf(" r30=%lx", reg_30);
    printf(" r31=%lx", reg_31);
    */
    printf("\n");
    fflush(stdout);
    print_pc_counter++;
}

//uint64_t chunk_done_counter = 0;
// struct timeval sync_start, sync_stop;
// uint64_t sync_duration = 0;
extern void _chunk_done()
{
    //chunk_done_counter++;
    //printf("chunk_done() counter=%lu\n", chunk_done_counter);
    //gettimeofday(&sync_start, NULL);
    __sync_synchronize();
    // gettimeofday(&sync_stop, NULL);
    // sync_duration += TimeDiff(sync_start, sync_stop);
    // printf("chunk_done() sync_duration=%lu\n", sync_duration);

    // Notify the caller that a new chunk is done and its trace is ready to be consumed
    assert(call_chunk_done);
    int result = sem_post(sem_chunk_done);
    if (result == -1)
    {
        printf("ERROR: Failed calling sem_post(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
}

extern void _realloc_trace (void)
{
    realloc_counter++;

    // Calculate new trace size
    uint64_t new_trace_size = trace_size * 2;

    // Extend the underlying file to the new size
    int result = ftruncate(shmem_output_fd, new_trace_size);
    if (result != 0)
    {
        printf("ERROR: realloc_trace() failed calling ftruncate(%s) of new size=%lu errno=%d=%s\n", shmem_output_name, new_trace_size, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Remap the memory
    void * new_address = mremap((void *)trace_address, trace_size, new_trace_size, 0);
    if ((uint64_t)new_address != trace_address)
    {
        printf("ERROR: realloc_trace() failed calling mremap() from size=%lu to %lu got new_address=%p errno=%d=%s\n", trace_size, new_trace_size, new_address, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Update trace global variables
    set_trace_size(new_trace_size);

#ifdef DEBUG
    if (verbose) printf("realloc_trace() realloc counter=%lu trace_address=0x%lx trace_size=%lu=%lx max_address=0x%lx trace_address_threshold=0x%lx chunk_size=%lu\n", realloc_counter, trace_address, trace_size, trace_size, trace_address + trace_size, trace_address_threshold, chunk_size);
#endif
}

/* Trace data structure
    [8B] Number of chunks: C

    Chunk 0:
        Start state:
            [8B] pc
            [8B] sp
            [8B] c
            [8B] step
            [8B] register[1]
            …
            [8B] register[31]
            [8B] register[32]
            [8B] register[33]
        Last state:
            [8B] c
        End:
            [8B] end
        Steps:
            [8B] steps = chunk size except for the last chunk
            [8B] mem_reads_size
            [8B] mem_reads[0]
            [8B] mem_reads[1]
            …
            [8B] mem_reads[mem_reads_size - 1]

    Chunk 1:
    …
    Chunk C-1:
    …
*/
void log_minimal_trace(void)
{
    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    printf("Allocated size = %lu B\n", pOutput[2]); // Allocated size [8]
    printf("Minimal trace used size = %lu B\n", pOutput[3]); // Minimal trace used size [8]

    printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)MEM_TRACE_ADDRESS;
    uint64_t number_of_chunks = trace[0];
    printf("Number of chunks=%lu\n", number_of_chunks);
    if (number_of_chunks > 1000000)
    {
        printf("ERROR: Number of chunks is too high=%lu\n", number_of_chunks);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        uint64_t i=0;
        printf("Chunk %lu (@=%p):\n", c, chunk);

        // Log current chunk start state
        printf("\tStart state:\n");
        printf("\t\tpc=0x%lx\n", chunk[i]);
        i++;
        printf("\t\tsp=0x%lx\n", chunk[i]);
        i++;
        printf("\t\tc=0x%lx\n", chunk[i]);
        i++;
        printf("\t\tstep=%lu\n", chunk[i]);
        i++;
        printf("\t\t");
        for (uint64_t r=1; r<34; r++)
        {
            printf("reg[%lu]=0x%lx ", r, chunk[i]);
            i++;
        }
        printf("\n");

        // Log current chunk last state
        printf("\tEnd state:\n");
        printf("\t\tc=0x%lx\n", chunk[i]);
        i++;
        // Log current chunk end
        printf("\t\tend=%lu\n", chunk[i]);
        i++;
        // Log current chunk steps
        printf("\t\tsteps=%lu\n", chunk[i]);
        i++;

        uint64_t mem_reads_size = chunk[i];
        printf("\t\tmem_reads_size=%lu\n", mem_reads_size);
        i++;
        if (mem_reads_size > 10000000)
        {
            printf("ERROR: Mem reads size is too high=%lu\n", mem_reads_size);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (trace_trace)
        {
            for (uint64_t m=0; m<mem_reads_size; m++)
            {
                printf("\t\tchunk[%lu].mem_reads[%lu]=%08lx\n", c, m, chunk[i]);
                i++;
            }
        }
        else
        {
            i += mem_reads_size;
        }

        //Set next chunk pointer
        chunk = chunk + i;
    }
    printf("Trace=%p chunk=%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}

void log_histogram(void)
{

    uint64_t *  pOutput = (uint64_t *)TRACE_ADDR;
    printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    printf("Allocated size = %lu B\n", pOutput[2]); // MT allocated size [8]
    printf("Steps = %lu B\n", pOutput[3]); // MT used size [8]

    printf("BIOS histogram:\n");
    uint64_t * trace = (uint64_t *)(TRACE_ADDR + 0x20);

    // BIOS
    uint64_t bios_size = trace[0];
    printf("BIOS size=%lu\n", bios_size);
    if (bios_size > 100000000)
    {
        printf("ERROR: Bios size is too high=%lu\n", bios_size);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (trace_trace)
    {
        uint64_t * bios = trace + 1;
        for (uint64_t i=0; i<bios_size; i++)
        {
            printf("%lu: pc=0x%lx multiplicity=%lu:\n", i, 0x1000 + (i*4), bios[i] );
        }
    }

    // Program
    uint64_t program_size = trace[bios_size + 1];
    printf("Program size=%lu\n", program_size);
    if (program_size > 100000000)
    {
        printf("ERROR: Program size is too high=%lu\n", program_size);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (trace_trace)
    {
        uint64_t * program = trace + 1 + bios_size + 1;
        for (uint64_t i=0; i<program_size; i++)
        {
            if (program[i] != 0)
            {
                printf("%lu: pc=0x%lx multiplicity=%lu:\n", i, 0x80000000 + i, program[i]);
            }
        }
    }

    printf("Histogram bios_size=%lu program_size=%lu\n", bios_size, program_size);
}

/* Trace data structure
    [8B] Number of chunks = C

    Chunk 0:
        [8B] mem_trace_size
        [7x8B] mem_trace[0]
        [7x8B] mem_trace[1]
        …
        [7x8B] mem_trace[mem_trace_size - 1]

    Chunk 1:
    …
    Chunk C-1:
    …
*/
void log_main_trace(void)
{
    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    printf("Allocated size = %lu B\n", pOutput[2]); // Allocated size [8]
    printf("Main trace used size = %lu B\n", pOutput[3]); // Main trace used size [8]

    printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)MEM_TRACE_ADDRESS;
    uint64_t number_of_chunks = trace[0];
    printf("Number of chunks=%lu\n", number_of_chunks);
    if (number_of_chunks > 1000000)
    {
        printf("ERROR: Number of chunks is too high=%lu\n", number_of_chunks);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        uint64_t i=0;
        printf("Chunk %lu:\n", c);

        uint64_t main_trace_size = chunk[i];
        printf("\tmem_reads_size=%lu\n", main_trace_size);
        i++;
        main_trace_size /= 7;
        if (main_trace_size > 10000000)
        {
            printf("ERROR: Main_trace size is too high=%lu\n", main_trace_size);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        if (trace_trace)
        {
            for (uint64_t m=0; m<main_trace_size; m++)
            {
                printf("\t\tchunk[%lu].main_trace[%lu]=[%lx,%lx,%lx,%lx,%lx,%lx,%lx]\n",
                    c,
                    m,
                    chunk[i],
                    chunk[i+1],
                    chunk[i+2],
                    chunk[i+3],
                    chunk[i+4],
                    chunk[i+5],
                    chunk[i+6]
                );
                i += 7;
            }
        }
        else
        {
            i += main_trace_size*7;
        }

        //Set next chunk pointer
        chunk = chunk + i;
    }
    printf("Trace=%p chunk=%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}

void buffer2file (const void * buffer_address, size_t buffer_length, const char * file_name)
{
    if (!file_name)
    {
        printf("ERROR: buffer2file() found invalid file_name\n");
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (!buffer_address)
    {
        printf("ERROR: buffer2file() found invalid buffer_address\n");
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    FILE * file = fopen(file_name, "wb");
    if (!file)
    {
        printf("ERROR: buffer2file() failed calling fopen(%s) errno=%d=%s\n", file_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    if (buffer_length > 0)
    {
        size_t bytes_written = fwrite(buffer_address, 1, buffer_length, file);
        if (bytes_written != buffer_length)
        {
            printf("ERROR: buffer2file() failed calling fwrite(%s) buffer_address=%p buffer_length=%lu errno=%d=%s\n", file_name, buffer_address, buffer_length, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            fclose(file);
            exit(-1);
        }
    }

    if (fclose(file) != 0)
    {
        printf("ERROR: buffer2file() failed calling fclose(%s) errno=%d=%s\n", file_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
}

/* Memory operations structure
    [8B] Number of chunks = C

    Chunk 0:
        [8b] end
        [8B] mem_op_trace_size
        [8B] mem_op_trace[0]
        [8B] mem_op_trace[1]
        …
        [8B] mem_op_trace[mem_op_trace_size - 1]

    Chunk 1:
    …
    Chunk C-1:
    …
*/
void log_mem_op(void)
{
    // Log header
    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    printf("Allocated size = %lu B\n", pOutput[2]); // Allocated size [8]
    printf("Memory operations trace used size = %lu B\n", pOutput[3]); // Main trace used size [8]

    printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)MEM_TRACE_ADDRESS;
    uint64_t number_of_chunks = trace[0];
    printf("Number of chunks=%lu\n", number_of_chunks);
    if (number_of_chunks > 1000000)
    {
        printf("ERROR: Number of chunks is too high=%lu\n", number_of_chunks);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        uint64_t i=0;
        printf("Chunk %lu:\n", c);

        uint64_t end = chunk[i];
        printf("\tend=%lu\n", end);
        i++;

        uint64_t mem_op_trace_size = chunk[i];
        printf("\tmem_op_trace_size=%lu\n", mem_op_trace_size);
        i++;
        if (mem_op_trace_size > 10000000)
        {
            printf("ERROR: Mem op trace size is too high=%lu\n", mem_op_trace_size);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        for (uint64_t m=0; m<mem_op_trace_size; m++)
        {
            uint64_t rest_are_zeros = (chunk[i] >> 49) & 0x1;
            uint64_t write = (chunk[i] >> 48) & 0x1;
            uint64_t width = (chunk[i] >> 32) & 0xF;
            uint64_t address = chunk[i] & 0xFFFFFFFF;
            bool inside_range =
                ((address >= RAM_ADDR) && (address < (RAM_ADDR + RAM_SIZE))) ||
                ((address >= ROM_ADDR) && (address < (ROM_ADDR + ROM_SIZE))) ||
                ((address >= INPUT_ADDR) && (address < (INPUT_ADDR + MAX_INPUT_SIZE)));
            if (trace_trace || !inside_range)
            {
                printf("\t\tchunk[%lu].mem_op_trace[%lu] = %016lx = rest_are_zeros=%lx, write=%lx, width=%lx, address=%lx%s\n",
                    c,
                    m,
                    chunk[i],
                    rest_are_zeros,
                    write,
                    width,
                    address,
                    inside_range ? "" : " ERROR!!!!!!!!!!!!!!"
                );
            }
            i += 1;
        }

        //Set next chunk pointer
        chunk = chunk + i;
    }
    printf("Trace=%p chunk=%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}

/* Memory trace structure (for 1 chunk)
    [8B] mem_trace_size
    [16B] mem_trace[0]
        [8B] mem operacion
            [4B] address (LE)
            [1B] width (1, 2, 4, 8) + write (0, 1) << 4
            [3B] 
    [16B] mem_trace[1]
    …
    [16B] mem_trace[mem_trace_size - 1]
*/
void log_mem_trace(void)
{
    printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)trace_address;
    printf("log_mem_trace() trace_address=%p\n", trace);
    uint64_t i=0;
    printf("Version = 0x%06lx\n", trace[0]); // Version, e.g. v1.0.0 [8]
    printf("Exit code = %lu\n", trace[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    printf("Allocated size = %lu B\n", trace[2]); // Allocated size [8]
    printf("Memory operations trace used size = %lu B\n", trace[3]); // Main trace used size [8]
    i += 4;
    uint64_t number_of_entries = trace[i];
    i++;
    printf("Trace size=%lu\n", number_of_entries);

    for (uint64_t m = 0; m < number_of_entries; m++)
    {
        uint64_t addr_step = trace[i];
        i++;

        // addr_step = [@0, @1, @2, @3, width + write<<4, supra_step]        
        uint64_t address = addr_step & 0xFFFFFFFF;
        uint64_t width = (addr_step >> (4*8)) & 0xF;
        uint64_t write = (addr_step >> ((4*8) + 4)) & 0x1;
        uint64_t micro_step = (addr_step >> (5*8)) & 0x3;
        uint64_t incremental_step = (addr_step >> ((5*8) + 2));
        bool address_is_inside_range =
            ((address >= RAM_ADDR) && (address < (RAM_ADDR + RAM_SIZE))) ||
            ((address >= ROM_ADDR) && (address < (ROM_ADDR + ROM_SIZE))) ||
            ((address >= INPUT_ADDR) && (address < (INPUT_ADDR + MAX_INPUT_SIZE)));
        bool width_is_valid = (width == 1) || (width == 2) || (width == 4) || (width == 8);
        bool bError = !(address_is_inside_range && width_is_valid);
        if (trace_trace || bError)
        {
            printf("\tmem_trace[%lu] = %016lx = [inc_step=%lu, u_step=%lu, write=%lx, width=%lx, address=%lx] %s\n",
                m,
                addr_step,
                incremental_step,
                micro_step,
                write,
                width,
                address,
                bError ? " ERROR!!!!!!!!!!!!!!" : ""
            );
        }

        // u-step:
        //   0: a=SRC_MEM
        //   1: b=SRC_MEM or b=SRC_IND
        //   2: precompiled_read
        //   3: c=STORE_MEM, c=STORE_IND or precompiled_write

        bool address_is_aligned = (address & 0x7) == 0;
        uint64_t aligned_address = address & 0xFFFFFFF8;
        uint64_t number_of_read_values = 0;
        uint64_t number_of_write_values = 0;

        switch (micro_step)
        {
            case 0: // a=SRC_MEM
            {
                assert_perror(width == 8);
                if (address_is_aligned)
                {
                    number_of_read_values = 1;
                }
                else
                {
                    number_of_read_values = 2;
                }
                break;
            }
            case 1: // b=SRC_MEM or b=SRC_IND
            {
                if (address_is_aligned)
                {
                    number_of_read_values = 1;
                }
                else
                {
                    if (((address + width - 1) & 0xFFFFFFF8) == aligned_address)
                    {
                        number_of_read_values = 1;
                    }
                    else
                    {
                        number_of_read_values = 2;
                    }
                }
                break;
            }
            case 2: // precompiled_read
            {
                assert_perror(width == 8);
                if (address_is_aligned)
                {
                    number_of_read_values = 1;
                }
                else
                {
                    number_of_read_values = 2;
                }
                break;
            }
            case 3: // c=STORE_MEM, c=STORE_IND or precompiled_write
            {
                if (address_is_aligned && (width == 8))
                {
                    number_of_read_values = 0;
                }
                else
                {
                    if (((address + width - 1) & 0xFFFFFFF8) == aligned_address)
                    {
                        number_of_read_values = 1;
                    }
                    else
                    {
                        number_of_read_values = 2;
                    }
                }
                number_of_write_values = 1;
                break;
            }
        }

        for (uint64_t r = 0; r < number_of_read_values; r++)
        {
            uint64_t value = trace[i];
            i++;
            m++;
            if (trace_trace)
            {
                printf("\t\tread_value[%lu] = 0x%lx\n", i, value);
            }
        }

        for (uint64_t w = 0; w < number_of_write_values; w++)
        {
            uint64_t value = trace[i];
            i++;
            m++;
            if (trace_trace)
            {
                printf("\t\twrite_value[%lu] = 0x%lx\n", i, value);
            }
        }
    }
    printf("Trace=%p number_of_entries=%lu\n", trace, number_of_entries);
}

void save_mem_op_to_files(void)
{
    // Log header
    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    printf("Version = 0x%06lx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    printf("Exit code = %lu\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    printf("Allocated size = %lu B\n", pOutput[2]); // Allocated size [8]
    printf("Memory operations trace used size = %lu B\n", pOutput[3]); // Main trace used size [8]

    printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)MEM_TRACE_ADDRESS;
    uint64_t number_of_chunks = trace[0];
    printf("Number of chunks=%lu\n", number_of_chunks);
    if (number_of_chunks > 1000000)
    {
        printf("ERROR: Number of chunks is too high=%lu\n", number_of_chunks);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        char file_name[256];
        sprintf(file_name, "/tmp/mem_count_data_%lu.bin", c);

        uint64_t i=0;
        uint64_t mem_op_trace_size = chunk[i];
        i++;
        if (mem_op_trace_size > 10000000)
        {
            printf("ERROR: Mem op trace size is too high=%lu\n", mem_op_trace_size);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        printf("Chunk %lu: file=%s length=%lu\n", c, file_name, mem_op_trace_size);

        buffer2file(&chunk[i], mem_op_trace_size * 8, file_name);

        //Set next chunk pointer
        chunk = chunk + mem_op_trace_size + 1;
    }
    printf("Trace=%p chunk=%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}

/* Trace data structure
    [8B] Number of elements

    A series of elements with the following structure:
        [8B] op: instruction opcode
        [8B] a: register a value
        [8B] b: register b value
        [8B] precompiled_memory_address: memory read address of the precompiled input data
*/
void log_chunk_player_main_trace(void)
{
    uint64_t * chunk = (uint64_t *)trace_address;
    uint64_t i = 0;

    printf("Version = 0x%06lx\n", chunk[0]); // Version, e.g. v1.0.0 [8]
    printf("Exit code = %lu\n", chunk[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    printf("Allocated size = %lu B\n", chunk[2]); // Allocated size [8]
    printf("Memory operations trace used size = %lu B\n", chunk[3]); // Main trace used size [8]
    i = 4;

    uint64_t mem_reads_size = chunk[i];
    i++;
    printf("mem_reads_size=%lu\n", mem_reads_size);
    if (mem_reads_size > 10000000)
    {
        printf("ERROR: Mem reads size is too high=%lu\n", mem_reads_size);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    //if (trace_trace)
    {
        for (uint64_t m=0; m<mem_reads_size; m++)
        {
            uint64_t op = chunk[i];
            if (trace_trace) printf("\tmem_reads[%lu] op=0x%lx\n", m, chunk[i]);
            i++;
            m++;
            if (op > 0xFF)
            {
                printf("ERROR!! Invalid op=%lu=0x%lx\n", op, op);
            }
            if (trace_trace) printf("\tmem_reads[%lu] a=0x%08lx\n", m, chunk[i]);
            i++;
            m++;
            if (trace_trace) printf("\tmem_reads[%lu] b=0x%08lx\n", m, chunk[i]);
            i++;
            m++;
            if (   (op == 0xf1) // Keccak
                || (op == 0xf9) // SHA256
                || (op == 0xf2) // Arith256
                || (op == 0xf3) // Arith256Mod
                || (op == 0xf4) // Secp256k1Add
                || (op == 0xf5) // Secp256k1Dbl
                )
            {
                if (trace_trace) printf("\tmem_reads[%lu] precompiled_address=%08lx\n", m, chunk[i]);
                i++;
                m++;
            }
        }
    }

    printf("Chunk=%p size=%lu\n", chunk, mem_reads_size);
}