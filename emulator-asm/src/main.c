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

#define DEBUG

// Assembly-provided functions
void emulator_start(void);
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
#define INITIAL_TRACE_SIZE (uint64_t)0x40000000 // 4GB

#define REG_ADDR (uint64_t)0x70000000
#define REG_SIZE (uint64_t)0x1000 // 4kB

#define TYPE_PING 1 // Ping
#define TYPE_PONG 2
#define TYPE_MT_REQUEST 3 // Minimal trace
#define TYPE_MT_RESPONSE 4
#define TYPE_RH_REQUEST 5 // ROM histogram
#define TYPE_RH_RESPONSE 6
#define TYPE_MO_REQUEST 7 // Memory opcode
#define TYPE_MO_RESPONSE 8
#define TYPE_SD_REQUEST 1000000 // Shutdown
#define TYPE_SD_RESPONSE 1000001

// Generation method
typedef enum {
    Fast = 0,
    MinimalTrace = 1,
    RomHistogram = 2,
    MainTrace = 3,
    ChunksOnly = 4,
    BusOp = 5,
    Zip = 6,
} GenMethod;
GenMethod gen_method = Fast;

// Service TCP parameters
#define SERVER_IP "127.0.0.1"  // Change to your server IP
uint16_t port = 0;

// Type of execution
bool server = false;
bool client = false;
bool chunk_done = false;
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
extern uint64_t MEM_TRACE_ADDRESS;
extern uint64_t MEM_CHUNK_ADDRESS;
extern uint64_t MEM_CHUNK_START_STEP;

uint64_t realloc_counter = 0;

extern void zisk_keccakf(uint64_t state[25]);

#define CHUNK_SIZE 1024*1024
uint64_t chunk_size = CHUNK_SIZE;
uint64_t chunk_size_mask = CHUNK_SIZE - 1;
uint64_t max_steps = 0xffffffffffffffff;

uint64_t initial_trace_size = INITIAL_TRACE_SIZE;
uint64_t trace_address = TRACE_ADDR;
uint64_t trace_size = INITIAL_TRACE_SIZE;

// Worst case: every chunk instruction is a keccak operation, with an input data of 200 bytes
#define MAX_CHUNK_TRACE_SIZE (CHUNK_SIZE * 200) + (44 * 8) + 32
uint64_t trace_address_threshold = TRACE_ADDR + INITIAL_TRACE_SIZE - MAX_CHUNK_TRACE_SIZE;

void parse_arguments(int argc, char *argv[]);
uint64_t TimeDiff(const struct timeval startTime, const struct timeval endTime);

void configure (void);
void client_run (void);
void server_setup (void);
void server_reset (void);
void server_run (void);
void server_cleanup (void);

void log_minimal_trace(void);
void log_histogram(void);
void log_main_trace(void);

int recv_all_with_timeout (int sockfd, void *buffer, size_t length, int flags, int timeout_sec);

// Configuration
bool output = true;
bool metrics = false;
bool trace = false;
bool trace_trace = false;
#ifdef DEBUG
bool verbose = false;
#endif
bool generate_minimal_trace = false;

// ROM histogram
bool generate_rom_histogram = false;
uint64_t histogram_size = 0;
uint64_t bios_size = 0;
uint64_t program_size = 0;

// Main trace
bool generate_main_trace = false;

// Chunks
bool generate_chunks = false;

// Fast
bool generate_fast = false;

// Zip
bool generate_zip = false;
uint64_t chunk_mask = 0x0; // 0, 1, 2, 3, 4, 5, 6 or 7
#define MAX_CHUNK_MASK 7

// Maximum length of the shared memory prefix, e.g. SHMZISK12345678
#define MAX_SHM_PREFIX_LENGTH 32

// Input shared memory
char * shmem_input_sufix = "_input";
char shmem_input_name[128];
int shmem_input_fd = -1;
uint64_t shmem_input_size = 0;
void * shmem_input_address = NULL;

// Output shared memory
char * shmem_output_sufix = "_output";
char shmem_output_name[128];
int shmem_output_fd = -1;

// Chunk done semaphore: notifies the caller when a new chunk has been processed
char * sem_chunk_done_sufix = "_semckd";
char sem_chunk_done_name[128];
sem_t * sem_chunk_done = NULL;

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
        client_run();
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
        printf("Failed calling socket() errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Forcefully attach socket to the port (avoid "address already in use")
    int opt = 1;
    result = setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR | SO_REUSEPORT, &opt, sizeof(opt));
    if (result != 0)
    {
        printf("Failed calling setsockopt() result=%d errno=%d=%s\n", result, errno, strerror(errno));
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
        printf("Failed calling bind() result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Start listening
    result = listen(server_fd, 5);
    if (result != 0)
    {
        printf("Failed calling listen() result=%d errno=%d=%s\n", result, errno, strerror(errno));
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
        printf("Calling accept()...\n");
        client_fd = accept(server_fd, (struct sockaddr *)&address, (socklen_t*)&addrlen);
        if (client_fd < 0)
        {
            printf("Failed calling accept() client_fd=%d errno=%d=%s\n", client_fd, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        if (verbose) printf("New client: %s:%d\n", inet_ntoa(address.sin_addr), ntohs(address.sin_port));

        // Configure linger to send data before closing the socket
        // struct linger linger_opt = {1, 5};  // Enable linger with 5s timeout
        // setsockopt(client_fd, SOL_SOCKET, SO_LINGER, &linger_opt, sizeof(linger_opt));
        // int cork = 0;
        // setsockopt(client_fd, IPPROTO_TCP, TCP_CORK, &cork, sizeof(cork));
        // // Disable Nagle algorithm
        // int flag = 1;
        // setsockopt(client_fd, IPPROTO_TCP, TCP_NODELAY, &flag, sizeof(flag));

        bool bShutdown = false;

        while (true)
        {
            // Read client request
            uint64_t request[5];
            ssize_t bytes_read = recv(client_fd, request, sizeof(request), MSG_WAITALL);
            if (bytes_read < 0)
            {
                printf("Failed calling recv() bytes_read=%ld errno=%d=%s\n", bytes_read, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                break;
            }
            if (bytes_read != sizeof(request))
            {
                printf("Failed calling recv() invalid bytes_read=%ld errno=%d=%s\n", bytes_read, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                break;
            }
            if (verbose) printf("recv() returned: %ld\n", bytes_read);

            uint64_t response[5];
            switch (request[0])
            {
                case TYPE_PING:
                {
                    if (verbose) printf("PING received\n");
                    response[0] = TYPE_PONG;
                    response[1] = gen_method;
                    response[2] = trace_size;
                    response[3] = 0;
                    response[4] = 0x0102030405060708;
                    break;
                }
                case TYPE_MT_REQUEST:
                {
                    if (verbose) printf("MINIMAL TRACE received\n");
                    if (gen_method == MinimalTrace)
                    {
                        server_run();

                        response[0] = TYPE_MT_RESPONSE;
                        response[1] = 0;
                        response[2] = trace_size;
                        response[3] = trace_size;
                        response[4] = 0;
                    }
                    else
                    {
                        response[0] = TYPE_MT_RESPONSE;
                        response[1] = 1;
                        response[2] = trace_size;
                        response[3] = trace_size;
                        response[4] = 0;
                    }
                    break;
                }
                case TYPE_SD_REQUEST:
                {
                    if (verbose) printf("SHUTDOWN received\n");
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
                    printf("invalid request id=%lu\n", request[0]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);                
                }
            }

            printf("size=%ld\n", sizeof(response));
            ssize_t bytes_sent = send(client_fd, response, sizeof(response), 0);
            if (bytes_sent != sizeof(response))
            {
                printf("Failed calling send() invalid bytes_sent=%ld errno=%d=%s\n", bytes_sent, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                break;
            }
            if (verbose) printf("Response sent to client\n");

            if (bShutdown)
            {
                break;
            }
        }

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
    char * usage = "Usage: ziskemuasm -s(server) -c(client) -f <input_file> -p <port_number> [--gen=0|--generate_fast] [--gen=1|--generate_minimal_trace] [--gen=2|--generate_rom_histogram] [--gen=3|--generate_main_trace] [--gen=4|--generate_chunks] [--gen=6|--generate_zip] [--chunk <chunk_number>] [--shutdows] [--mt <number_of_mt_requests>] [-o output off] [-m metrics on] [-t trace on] [-tt trace on] [-h/--help print this]";
#ifdef DEBUG
    printf("%s [-v verbose on] [-k keccak trace on]\n", usage);
#else
    printf("%s\n", usage);
#endif
}

void parse_arguments(int argc, char *argv[])
{
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
                generate_fast = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=1") == 0) || (strcmp(argv[i], "--generate_minimal_trace") == 0))
            {
                gen_method = MinimalTrace;
                generate_minimal_trace = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=2") == 0) || (strcmp(argv[i], "--generate_rom_histogram") == 0))
            {
                gen_method = RomHistogram;
                generate_rom_histogram = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=3") == 0) || (strcmp(argv[i], "--generate_main_trace") == 0))
            {
                gen_method = MainTrace;
                generate_main_trace = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=4") == 0) || (strcmp(argv[i], "--generate_chunks") == 0))
            {
                gen_method = ChunksOnly;
                generate_chunks = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=6") == 0) || (strcmp(argv[i], "--generate_zip") == 0))
            {
                gen_method = Zip;
                generate_zip = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if (strcmp(argv[i], "-o") == 0)
            {
                output = false;
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
#ifdef DEBUG
                verbose = true;
#else
                printf("Verbose option -v is only available in debug compilation\n");
                print_usage();
                exit(-1);
#endif
                continue;
            }
            if (strcmp(argv[i], "-k") == 0)
            {
#ifdef DEBUG
                keccak_metrics = true;
#else
                printf("Keccak metrics option -k is only available in debug compilation\n");
                print_usage();
                exit(-1);
#endif
                continue;
            }
            if (strcmp(argv[i], "-h") == 0)
            {
                print_usage();
                continue;
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
                    printf("Detected argument -i in the last position; please provide input file after it\n");
                    print_usage();
                    exit(-1);
                }
                if (strlen(argv[i]) > 4095)
                {
                    printf("Detected argument -i but next argumet is too long\n");
                    print_usage();
                    exit(-1);
                }
                strcpy(input_file, argv[i]);
                continue;
            }

            if (strcmp(argv[i], "--chunk") == 0)
            {
                i++;
                if (i >= argc)
                {
                    printf("Detected argument -c in the last position; please provide chunk number after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                chunk_mask = strtoul(argv[i], &endptr, 10);

                // Check for errors
                if (errno == ERANGE) {
                    printf("Error: Chunk number is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argv[i]) {
                    printf("Error: No digits found while parsing chunk number\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("Error: Extra characters after chunk number: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else if (chunk_mask > MAX_CHUNK_MASK) {
                    printf("Error: Invalid chunk number: %lu\n", chunk_mask);
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
                    printf("Detected argument -mt in the last position; please provide chunk number after it\n");
                    print_usage();
                    exit(-1);
                }
                errno = 0;
                char *endptr;
                number_of_mt_requests = strtoul(argv[i], &endptr, 10);

                // Check for errors
                if (errno == ERANGE) {
                    printf("Error: Number of MT requests is too large\n");
                    print_usage();
                    exit(-1);
                } else if (endptr == argv[i]) {
                    printf("Error: No digits found while parsing number of MT requests\n");
                    print_usage();
                    exit(-1);
                } else if (*endptr != '\0') {
                    printf("Error: Extra characters after number of MT requests: %s\n", endptr);
                    print_usage();
                    exit(-1);
                } else if (number_of_mt_requests > 1000000) {
                    printf("Error: Invalid number of MT requests: %lu\n", number_of_mt_requests);
                    print_usage();
                    exit(-1);
                } else {
                    printf("Got number of MT requests= %lu\n", number_of_mt_requests);
                }
                continue;
            }
            printf("Unrecognized argument: %s\n", argv[i]);
            print_usage();
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    }

    // Check that only one generation method was selected as an argument
    if (number_of_selected_generation_methods != 1)
    {
        printf("Invalid arguments: select 1 generation method, and only one\n");
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
        printf("Inconsistency: C generation method is %u but ASM generation method is %lu\n",
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
        printf("Inconsistency: both server and client at the same time is not possible\n");
        print_usage();
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (!server && !client)
    {
        printf("Inconsistency: select server or client\n");
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
            strcpy(shmem_input_name, "ZISKFT_input");
            strcpy(shmem_output_name, "ZISKFT_output");
            strcpy(sem_chunk_done_name, "");
            port = 23115;
            break;
        }
        case MinimalTrace:
        {
            strcpy(shmem_input_name, "ZISKMT_input");
            strcpy(shmem_output_name, "ZISKMT_output");
            strcpy(sem_chunk_done_name, "ZISKMT_chunk_done");
            chunk_done = true;
            port = 23115;
            break;
        }
        case RomHistogram:
        {
            strcpy(shmem_input_name, "ZISKRH_input");
            strcpy(shmem_output_name, "ZISKRH_output");
            strcpy(sem_chunk_done_name, "");
            port = 23116;
            break;
        }
        case MainTrace:
        {
            strcpy(shmem_input_name, "ZISKMA_input");
            strcpy(shmem_output_name, "ZISKMA_output");
            strcpy(sem_chunk_done_name, "ZISKMA_chunk_done");
            chunk_done = true;
            port = 23115;
            break;
        }
        case ChunksOnly:
        {
            strcpy(shmem_input_name, "ZISKCH_input");
            strcpy(shmem_output_name, "ZISKCH_output");
            strcpy(sem_chunk_done_name, "ZISKCH_chunk_done");
            chunk_done = true;
            port = 23115;
            break;
        }
        case BusOp:
        {
            strcpy(shmem_input_name, "ZISKBO_input");
            strcpy(shmem_output_name, "ZISKBO_output");
            strcpy(sem_chunk_done_name, "ZISKBO_chunk_done");
            chunk_done = true;
            port = 23115;
            break;
        }
        case Zip:
        {
            strcpy(shmem_input_name, "ZISKZP_input");
            strcpy(shmem_output_name, "ZISKZP_output");
            strcpy(sem_chunk_done_name, "ZISKZP_chunk_done");
            chunk_done = true;
            port = 23115;
            break;
        }
        default:
        {
            printf("Invalid gen_method = %u\n", gen_method);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    }

#ifdef DEBUG
    if (verbose) printf("ziskemuasm configuration: gen_method=%u port=%u shmem_input=%s shmem_output=%s sem_chunk_done=%s\n", gen_method, port, shmem_input_name, shmem_output_name, sem_chunk_done_name);
#endif
}

void client_run (void)
{
    assert(client);
    assert(!server);

    int result;

    /************************/
    /* Read input file data */
    /************************/

#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif

    // Open input file
    FILE * input_fp = fopen(input_file, "r");
    if (input_fp == NULL)
    {
        printf("Failed calling fopen(%s) errno=%d=%s; does it exist?\n", input_file, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Get input file size
    if (fseek(input_fp, 0, SEEK_END) == -1)
    {
        printf("Failed calling fseek(%s) errno=%d=%s\n", input_file, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    long input_data_size = ftell(input_fp);
    if (input_data_size == -1)
    {
        printf("Failed calling ftell(%s) errno=%d=%s\n", input_file, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Go back to the first byte
    if (fseek(input_fp, 0, SEEK_SET) == -1)
    {
        printf("Failed calling fseek(%s, 0) errno=%d=%s\n", input_file, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Check the input data size is inside the proper range
    if (input_data_size > (MAX_INPUT_SIZE - 16))
    {
        printf("Size of input file (%s) is too long (%lu)\n", input_file, input_data_size);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Open input shared memory
    shmem_input_fd = shm_open(shmem_input_name, O_RDWR, 0644);
    if (shmem_input_fd < 0)
    {
        printf("Failed calling shm_open(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Map the shared memory object into the process address space
    shmem_input_address = mmap(NULL, MAX_INPUT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, shmem_input_fd, 0);
    if (shmem_input_address == MAP_FAILED)
    {
        printf("Failed calling mmap(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
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
        printf("Input read (%lu) != input file size (%lu)\n", input_read, input_data_size);
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
        printf("Failed calling munmap(input) errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    printf("client (input): done in %lu us\n", duration);
#endif

    /*************************/
    /* Connect to the server */
    /*************************/
    
    // Create socket to connect to server
    int socket_fd;
    socket_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (socket_fd < 0)
    {
        printf("socket() failed socket_fd=%d errno=%d=%s\n", socket_fd, errno, strerror(errno));
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
        printf("inet_pton() failed.  Invalid address/Address not supported result=%d errno=%d=%s\n", result, errno, strerror(errno));
        exit(EXIT_FAILURE);
    }

    // Connect to server
    result = connect(socket_fd, (struct sockaddr *)&server_addr, sizeof(server_addr));
    if (result < 0)
    {
        printf("connect() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        exit(EXIT_FAILURE);
    }

    // Request and response
    uint64_t request[5];
    uint64_t response[5];

    /********/
    /* Ping */
    /********/

#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif

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
        printf("send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Read server response
    ssize_t bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
    if (bytes_received < 0)
    {
        printf("recv_all_with_timeout() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (bytes_received != sizeof(response))
    {
        printf("recv_all_with_timeout() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (response[0] != TYPE_PONG)
    {
        printf("recv_all_with_timeout() returned unexpected type=%lu\n", response[0]);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (response[1] != 1)
    {
        printf("recv_all_with_timeout() returned unexpected gen_method=%lu\n", response[1]);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    printf("client (PING): done in %lu us\n", duration);
#endif

    /*****************/
    /* Minimal trace */
    /*****************/
    for (uint64_t i=0; i<number_of_mt_requests; i++)
    {
#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif

    // Prepare message to send
    request[0] = TYPE_MT_REQUEST;
    request[1] = 1024*1024; // chunk_len
    request[2] = 0xFFFFFFFF; // max_steps
    request[3] = 0;
    request[4] = 0;

    // Send data to server
    result = send(socket_fd, request, sizeof(request), 0);
    if (result < 0)
    {
        printf("send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Read server response
    bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
    if (bytes_received < 0)
    {
        printf("recv_all_with_timeout() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (bytes_received != sizeof(response))
    {
        printf("recv_all_with_timeout() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (response[0] != TYPE_MT_RESPONSE)
    {
        printf("recv_all_with_timeout() returned unexpected type=%lu\n", response[0]);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (response[1] != 0)
    {
        printf("recv_all_with_timeout() returned unexpected result=%lu\n", response[1]);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    
#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    printf("client (MT): done in %lu us\n", duration);
#endif
    } // number_of_mt_requests

    /************/
    /* Shutdown */
    /************/

    if (do_shutdown)
    {

#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif

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
        printf("send() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Read server response
    bytes_received = recv(socket_fd, response, sizeof(response), MSG_WAITALL);
    if (bytes_received < 0)
    {
        printf("recv_all_with_timeout() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (bytes_received != sizeof(response))
    {
        printf("recv_all_with_timeout() returned bytes_received=%ld errno=%d=%s\n", bytes_received, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (response[0] != TYPE_SD_RESPONSE)
    {
        printf("recv_all_with_timeout() returned unexpected type=%lu\n", response[0]);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    
#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    printf("client (SD): done in %lu us\n", duration);
#endif

    } // do_shutdown

    /***********/
    /* Cleanup */
    /***********/

    // Close the socket
    close(socket_fd);
}

void server_setup (void)
{
    assert(server);
    assert(!client);

    int result;

    /*******/
    /* ROM */
    /*******/
#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif
    void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED | MAP_LOCKED, -1, 0);
#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
#endif
    if (pRom == NULL)
    {
        printf("Failed calling mmap(rom) errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if ((uint64_t)pRom != ROM_ADDR)
    {
        printf("Called mmap(rom) but returned address = 0x%p != 0x%lx\n", pRom, ROM_ADDR);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
#ifdef DEBUG
    if (verbose) printf("mmap(rom) returned 0x%p in %lu us\n", pRom, duration);
#endif

    /*********/
    /* INPUT */
    /*********/

    // Make sure the input shared memory is deleted
    shm_unlink(shmem_input_name);

    // Create the input shared memory
    shmem_input_fd = shm_open(shmem_input_name, O_RDWR | O_CREAT, 0666);
    if (shmem_input_fd < 0)
    {
        printf("Failed calling shm_open(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Size it
    result = ftruncate(shmem_input_fd, MAX_INPUT_SIZE);
    if (result != 0)
    {
        printf("Failed calling ftruncate(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Map input address space
#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif
    void * pInput = mmap((void *)INPUT_ADDR, MAX_INPUT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | /*MAP_ANONYMOUS |*/ MAP_FIXED | MAP_LOCKED, shmem_input_fd, 0);
#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
#endif
    if (pInput == NULL)
    {
        printf("Failed calling mmap(input) errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if ((uint64_t)pInput != INPUT_ADDR)
    {
        printf("Called mmap(pInput) but returned address = 0x%p != 0x%lx\n", pInput, INPUT_ADDR);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
#ifdef DEBUG
    if (verbose) printf("mmap(input) returned 0x%p in %lu us\n", pInput, duration);
#endif

    /*******/
    /* RAM */
    /*******/

#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif
    void * pRam = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED | MAP_LOCKED, -1, 0);
#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
#endif
    if (pRam == NULL)
    {
        printf("Failed calling mmap(ram) errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if ((uint64_t)pRam != RAM_ADDR)
    {
        printf("Called mmap(ram) but returned address = 0x%p != 0x%08lx\n", pRam, RAM_ADDR);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
#ifdef DEBUG
    if (verbose) printf("mmap(ram) returned 0x%p in %lu us\n", pRam, duration);
#endif

    /*********/
    /* TRACE */
    /*********/

    if (generate_rom_histogram)
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

    if (generate_minimal_trace || generate_rom_histogram || generate_main_trace || generate_zip)
    {
        // Make sure the output shared memory is deleted
        shm_unlink(shmem_output_name);

        // Create the output shared memory
        shmem_output_fd = shm_open(shmem_output_name, O_RDWR | O_CREAT, 0644);
        if (shmem_output_fd < 0)
        {
            printf("Failed calling shm_open(%s) errno=%d=%s\n", shmem_output_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Size it
        result = ftruncate(shmem_output_fd, trace_size);
        if (result != 0)
        {
            printf("Failed calling ftruncate(%s) errno=%d=%s\n", shmem_output_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }

        // Map it to the trace address
#ifdef DEBUG
        gettimeofday(&start_time, NULL);
#endif
        void * pTrace = mmap((void *)TRACE_ADDR, trace_size, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | MAP_LOCKED, shmem_output_fd, 0);
#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
#endif
        if (pTrace == NULL)
        {
            printf("Failed calling mmap(pTrace) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if ((uint64_t)pTrace != TRACE_ADDR)
        {
            printf("Called mmap(trace) but returned address = 0x%p != 0x%lx\n", pTrace, TRACE_ADDR);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    #ifdef DEBUG
        if (verbose) printf("mmap(trace) returned 0x%p in %lu us\n", pTrace, duration);
    #endif
    }

    /******************/
    /* SEM CHUNK DONE */
    /******************/

    sem_chunk_done = sem_open(sem_chunk_done_name, O_CREAT, 0644, 1);
    if (sem_chunk_done == SEM_FAILED)
    {
        printf("Failed calling sem_open(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
}

void server_reset (void)
{
    // Reset RAM data for next emulation
#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif
    memset((void *)RAM_ADDR, 0, RAM_SIZE);
#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    if (verbose) printf("memset(ram) in %lu us\n", duration);
#endif

    // Reset trace
    // Init output header data
    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    pOutput[0] = 0x000100; // Version, e.g. v1.0.0 [8]
    pOutput[1] = 1; // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    pOutput[2] = trace_size;
    // MT allocated size [8] -> to be updated after completion
    // MT used size [8] -> to be updated after completion
}

void server_run (void)
{
    /*******/
    /* ASM */
    /*******/

    uint64_t * pInput = (uint64_t *)INPUT_ADDR;

    // Call emulator assembly code
    gettimeofday(&start_time,NULL);
    emulator_start();
    gettimeofday(&stop_time,NULL);
    assembly_duration = TimeDiff(start_time, stop_time);

    uint64_t final_trace_size = MEM_CHUNK_ADDRESS - MEM_TRACE_ADDRESS;

    if ( metrics
#ifdef DEBUG
        || keccak_metrics
#endif
        )
    {
        uint64_t duration = assembly_duration;
        uint64_t steps = MEM_STEP;
        uint64_t end = MEM_END;
        uint64_t step_duration_ns = steps == 0 ? 0 : (duration * 1000) / steps;
        uint64_t step_tp_sec = duration == 0 ? 0 : steps * 1000000 / duration;
        uint64_t final_trace_size_percentage = (final_trace_size * 100) / trace_size;
#ifdef DEBUG
        printf("Duration = %lu us, Keccak counter = %lu, realloc counter = %lu, steps = %lu, step duration = %lu ns, tp = %lu steps/s, trace size = 0x%lx - 0x%lx = %lu B(%lu%%), end=%lu\n",
            duration,
            keccak_counter,
            realloc_counter,
            steps,
            step_duration_ns,
            step_tp_sec,
            MEM_CHUNK_ADDRESS,
            MEM_TRACE_ADDRESS,
            final_trace_size,
            final_trace_size_percentage,
            end);
        if (keccak_metrics)
        {
            uint64_t keccak_percentage = duration == 0 ? 0 : (keccak_duration * 100) / duration;
            uint64_t single_keccak_duration_ns = keccak_counter == 0 ? 0 : (keccak_duration * 1000) / keccak_counter;
            printf("Keccak counter = %lu, duration = %lu us, single keccak duration = %lu ns, percentage = %lu \n", keccak_counter, keccak_duration, single_keccak_duration_ns, keccak_percentage);
        }
#else
        printf("Duration = %lu us, realloc counter = %lu, steps = %lu, step duration = %lu ns, tp = %lu steps/s, trace size = 0x%lx - 0x%lx = %lu B(%lu%%), end=%lu\n",
            duration,
            realloc_counter,
            steps,
            step_duration_ns,
            step_tp_sec,
            MEM_CHUNK_ADDRESS,
            MEM_TRACE_ADDRESS,
            final_trace_size,
            final_trace_size_percentage,
            end);
#endif
        if (generate_rom_histogram)
        {
            printf("Rom histogram size=%lu\n", histogram_size);
        }
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
    if (generate_minimal_trace || generate_rom_histogram || generate_zip)
    {
        uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
        pOutput[0] = 0x000100; // Version, e.g. v1.0.0 [8]
        pOutput[1] = 0; // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
        pOutput[2] = trace_size; // MT allocated size [8]
        //assert(final_trace_size > 32);
        if (generate_minimal_trace || generate_zip)
        {
            pOutput[3] = final_trace_size; // MT used size [8]
        }
        else
        {
            pOutput[3] = MEM_STEP;
            pOutput[4] = bios_size;
            pOutput[4 + bios_size + 1] = program_size;
        }
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

    // Log trace
    if ((generate_minimal_trace || generate_zip) && trace)
    {
        log_minimal_trace();
    }
    if (generate_rom_histogram && trace)
    {
        log_histogram();
    }
    if (generate_main_trace && trace)
    {
        log_main_trace();
    }
}

void server_cleanup (void)
{
    // Cleanup ROM
    int result = munmap((void *)ROM_ADDR, ROM_SIZE);
    if (result == -1)
    {
        printf("Failed calling munmap(rom) errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Cleanup RAM
    result = munmap((void *)RAM_ADDR, RAM_SIZE);
    if (result == -1)
    {
        printf("Failed calling munmap(ram) errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Cleanup INPUT
    result = munmap((void *)INPUT_ADDR, MAX_INPUT_SIZE);
    if (result == -1)
    {
        printf("Failed calling munmap(input) errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    result = shm_unlink(shmem_input_name);
    if (result == -1)
    {
        printf("Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Cleanup trace
    result = munmap((void *)TRACE_ADDR, trace_size);
    if (result == -1)
    {
        printf("Failed calling munmap(trace) for size=%lu errno=%d=%s\n", trace_size, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    result = shm_unlink(shmem_output_name);
    if (result == -1)
    {
        printf("Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_output_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Cleanup semaphore
    if (chunk_done)
    {
        result = sem_close(sem_chunk_done);
        if (result == -1)
        {
            printf("Failed calling sem_close(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_chunk_done_name);
        if (result == -1)
        {
            printf("Failed calling sem_unlink(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        }
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

extern void _chunk_done()
{
    // Notify the caller that a new chunk is done and its trace is ready to be consumed
    assert((gen_method == MinimalTrace) || (gen_method == MainTrace) || (gen_method == Zip) || (gen_method == ChunksOnly));
    int result = sem_post(sem_chunk_done);
    if (result == -1)
    {
        printf("Failed calling sem_post(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
}

extern void _realloc_trace (void)
{
    realloc_counter++;
    //printf("realloc_trace() realloc counter=%d trace_address=0x%08x trace_size=%d\n", realloc_counter, trace_address, trace_size);

    // Calculate new trace size
    uint64_t new_trace_size = trace_size * 2;

    // Extend the underlying file to the new size
    int result = ftruncate(shmem_output_fd, new_trace_size);
    if (result != 0)
    {
        printf("realloc_trace() failed calling ftruncate(%s) of new size=%lu errno=%d=%s\n", shmem_output_name, new_trace_size, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Remap the memory
    void * new_address = mremap((void *)trace_address, trace_size, new_trace_size, 0);
    if ((uint64_t)new_address != trace_address)
    {
        printf("realloc_trace() failed calling mremap() from size=%lu to %lu got new_address=0x%p errno=%d=%s\n", trace_size, new_trace_size, new_address, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Update trace global variables
    trace_size = new_trace_size;
    trace_address_threshold = TRACE_ADDR + trace_size - MAX_CHUNK_TRACE_SIZE;

    ((uint64_t *)new_address)[2] = trace_size;
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
        printf("Number of chunks is too high=%lu\n", number_of_chunks);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        uint64_t i=0;
        printf("Chunk %lu:\n", c);

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
        for (uint64_t r=1; r<34; r++)
        {
            printf("\t\tregister[%lu]=0x%lx\n", r, chunk[i]);
            i++;
        }

        // Log current chunk last state
        printf("\tLast state:\n");
        printf("\t\tc=0x%lx\n", chunk[i]);
        i++;

        // Log current chunk end
        printf("\tEnd:\n");
        printf("\t\tend=%lu\n", chunk[i]);
        i++;

        // Log current chunk steps
        printf("\tSteps:\n");
        printf("\t\tsteps=%lu\n", chunk[i]);
        i++;
        uint64_t mem_reads_size = chunk[i];
        printf("\t\tmem_reads_size=%lu\n", mem_reads_size);
        i++;
        if (mem_reads_size > 10000000)
        {
            printf("Mem reads size is too high=%lu\n", mem_reads_size);
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
    printf("Trace=0x%p chunk=0x%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
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
        printf("Bios size is too high=%lu\n", bios_size);
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
        printf("Program size is too high=%lu\n", program_size);
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
        printf("Number of chunks is too high=%lu\n", number_of_chunks);
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
            printf("Main_trace size is too high=%lu\n", main_trace_size);
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
    printf("Trace=0x%p chunk=0x%p size=%lu\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}
