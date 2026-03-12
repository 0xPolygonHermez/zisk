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
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netinet/tcp.h>
#include <sys/file.h>
#include <time.h>
#include "constants.hpp"
#include "emu.hpp"
#include "asm_provided.hpp"
#include "globals.hpp"
#include "configuration.hpp"
#include "server.hpp"
#include "client.hpp"
#include "trace.hpp"

// Returns the acronym of the generation method, used for logging and file naming
const char * gen_method_acronym(GenMethod method)
{
    switch (method)
    {
        case Fast: return "FT";
        case MinimalTrace: return "MT";
        case RomHistogram: return "RH";
        case MainTrace: return "MA";
        case ChunksOnly: return "CO";
        //case BusOp: return "bus-op";
        case Zip: return "ZP";
        case MemOp: return "MO";
        case ChunkPlayerMTCollectMem: return "CPM";
        case MemReads: return "MR";
        case ChunkPlayerMemReadsCollectMain: return "CPMCM";
        default: return "?";
    }
}

// To be used when calculating total duration
struct timeval total_start_time;
struct timeval total_stop_time;
uint64_t total_duration;

// Checks if a number is a power of two, used to validate the max steps and chunk size provided by the client
bool is_power_of_two (uint64_t number)
{
    return (number != 0) && ((number & (number - 1)) == 0);
}

/*************/
/* MAX STEPS */
/*************/

// Sets the maximum number of steps provided by the client in the request
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

/**************/
/* CHUNK SIZE */
/**************/

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
    trace_address_threshold = TRACE_ADDR + trace_size - MAX_CHUNK_TRACE_SIZE;
}

//#define USE_FILE_LOCK

#ifdef USE_FILE_LOCK

void file_lock(void);

// File lock name, used to lock a file that indicates that the assembly emulator process is running,
// to prevent multiple instances of the server from running at the same time.
int file_lock_fd = -1;

#endif // USE_FILE_LOCK

// Process id
int process_id = 0;

#ifdef ASM_PRECOMPILE_CACHE
bool precompile_cache_enabled = false;
#endif

/********/
/* MAIN */
/********/

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

    // Get precompiled results configuration
    uint64_t precompile_results = get_precompile_results();
    if (precompile_results == 1) {
        precompile_results_enabled = true;
    } else {
        precompile_results_enabled = false;
    }

    // Parse arguments
    parse_arguments(argc, argv);

    // Redirect output to file if requested
    if (redirect_output_to_file)
    {
        char redirect_output_file[256];
        snprintf(redirect_output_file, sizeof(redirect_output_file), "/tmp/%s_%s_output.txt", shm_prefix, gen_method_acronym(gen_method));

        // Redirect stdout to file
        FILE * file_pointer = freopen(redirect_output_file, "w", stdout);
        if (file_pointer == NULL)
        {
            printf("ERROR: Failed to redirect stdout to file %s\n", redirect_output_file);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        
        // Redirect stderr to the same file
        file_pointer = freopen(redirect_output_file, "a", stderr);
        if (file_pointer == NULL)
        {
            printf("ERROR: Failed to redirect stderr to file %s\n", redirect_output_file);
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
    }

    // Configure based on parguments
    configure();

#ifdef USE_FILE_LOCK
    // Lock file
    if (server)
    {
        file_lock();
    }
#endif

    // Send a message to stderr
    // fprintf(stderr, "%s stderr test (not an error): Starting Ziskemu ASM emulator process id=%d server=%d client=%d gen_method=%d port=%u\n", log_name, process_id, server, client, gen_method, port);

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
    server_reset_fast();
    server_reset_slow();
    server_reset_trace();

    // Create socket file descriptor
    int server_fd;
    server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd == 0)
    {
        printf("%s ERROR: Failed calling socket() errno=%d=%s\n", log_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Forcefully attach socket to the port (avoid "address already in use")
    int opt = 1;
    result = setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR | SO_REUSEPORT, &opt, sizeof(opt));
    if (result != 0)
    {
        printf("%s ERROR: Failed calling setsockopt() result=%d errno=%d=%s\n", log_name, result, errno, strerror(errno));
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
        printf("%s ERROR: Failed calling bind() result=%d errno=%d=%s\n", log_name, result, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Start listening
    result = listen(server_fd, 5);
    if (result != 0)
    {
        printf("%s ERROR: Failed calling listen() result=%d errno=%d=%s\n", log_name, result, errno, strerror(errno));
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
        if (!silent)
        {
            printf("%s Waiting for incoming connections to port %u...\n", log_name, port);
            fflush(stdout);
            fflush(stderr);
        }
        client_fd = accept(server_fd, (struct sockaddr *)&address, (socklen_t*)&addrlen);
        if (client_fd < 0)
        {
            printf("ERROR: Failed calling accept() client_fd=%d errno=%d=%s\n", client_fd, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
#ifdef DEBUG
        if (verbose) printf("%s New client: %s:%d\n", log_name, inet_ntoa(address.sin_addr), ntohs(address.sin_port));
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
                printf("%s ERROR: Failed calling recv() bytes_read=%ld errno=%d=%s\n", log_name, bytes_read, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                break;
            }
            if (bytes_read != sizeof(request))
            {
                if ((errno != 0) && (errno != 2))
                {
                    printf("%s WARNING: Failed calling recv() invalid bytes_read=%ld errno=%d=%s\n", log_name, bytes_read, errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                }
                break;
            }
#ifdef DEBUG
            if (verbose)
            {
                printf("%s recv() returned: %ld\n", log_name, bytes_read);
                fflush(stdout);
                fflush(stderr);
            }
#endif
            if (verbose)
            {
                printf("%s recv()'d request=[%lu, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n", log_name, request[0], request[1], request[2], request[3], request[4]);
                fflush(stdout);
                fflush(stderr);
            }

            uint64_t response[5];
            bReset = false;
            switch (request[0])
            {
                case TYPE_PING:
                {
#ifdef DEBUG
                    if (verbose) printf("%s PING received\n", log_name);
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
                    if (verbose) printf("%s MINIMAL TRACE received\n", log_name);
#endif
                    if (gen_method == MinimalTrace)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        server_reset_fast();

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
                    if (verbose) printf("%s ROM HISTOGRAM received\n", log_name);
#endif
                    if (gen_method == RomHistogram)
                    {
                        set_max_steps(request[1]);

                        server_run();

                        server_reset_fast();

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
                    if (verbose) printf("%s MEMORY OPERATIONS received\n", log_name);
#endif
                    if (gen_method == MemOp)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        server_reset_fast();

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
                    if (verbose) printf("%s MAIN TRACE received\n", log_name);
#endif
                    if (gen_method == MainTrace)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        server_reset_fast();

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
                    if (verbose) printf("%s COLLECT MEMORY received\n", log_name);
#endif
                    if (gen_method == ChunkPlayerMTCollectMem)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);
                        chunk_player_address = request[3];
                        uint64_t * pChunk = (uint64_t *)chunk_player_address;
                        print_pc_counter = pChunk[3];

                        server_run();

                        server_reset_fast();

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
                    if (verbose) printf("%s FAST received\n", log_name);
#endif
                    if (gen_method == Fast)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        server_reset_fast();

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
                    if (verbose) printf("%s MEMORY READS received\n", log_name);
#endif
                    if (gen_method == MemReads)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);

                        server_run();

                        server_reset_fast();

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
                    if (verbose) printf("%s COLLECT MAIN received\n", log_name);
#endif
                    if (gen_method == ChunkPlayerMemReadsCollectMain)
                    {
                        set_max_steps(request[1]);
                        set_chunk_size(request[2]);
                        chunk_player_address = request[3];
                        uint64_t * pChunk = (uint64_t *)chunk_player_address;
                        print_pc_counter = pChunk[3];

                        server_run();

                        server_reset_fast();

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
                    if (!silent) printf("%s SHUTDOWN received\n", log_name);
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
                    printf("%s ERROR: Invalid request id=%lu\n", log_name, request[0]);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);                
                }
            }

            if (verbose)
            {
                printf("%s send()'ing response=[%lu, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n", log_name, response[0], response[1], response[2], response[3], response[4]);
                fflush(stdout);
                fflush(stderr);
            }

            ssize_t bytes_sent = send(client_fd, response, sizeof(response), MSG_WAITALL);
            if (bytes_sent != sizeof(response))
            {
                printf("%s ERROR: Failed calling send() invalid bytes_sent=%ld errno=%d=%s\n", log_name, bytes_sent, errno, strerror(errno));
                fflush(stdout);
                fflush(stderr);
                break;
            }
//#ifdef DEBUG
            else if (verbose)
            {
                printf("Response sent to client\n");
                fflush(stdout);
                fflush(stderr);
            }
//#endif
            if (bReset)
            {
                server_reset_slow();
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

/*************/
/* FILE LOCK */
/*************/

#ifdef USE_FILE_LOCK

// Lock file exclusively to ensure that only one instance of the program is running at a time
void file_lock(void)
{
    // Open (or create) the lock file. We don't need to write to it.
    file_lock_fd = open(file_lock_name, O_CREAT | O_RDONLY, 0644);
    if (file_lock_fd == -1) {
        printf("ERROR: file_lock() failed calling open(%s) errno=%d=%s\n", file_lock_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(1);
    }

    // Try to acquire an exclusive lock, non-blocking.
    if (flock(file_lock_fd, LOCK_EX | LOCK_NB) == -1) {
        // If we fail to get the lock, another instance is running.
        printf("ERROR: Another instance of this program is already running.\n");
        fflush(stdout);
        fflush(stderr);
        exit(1);
    }
}

#endif // USE_FILE_LOCK