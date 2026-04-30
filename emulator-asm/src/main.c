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
#include "log.hpp"

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
        asm_printf("ERROR: set_max_steps() got a new max steps = %lu that is not a power of two\n", new_max_steps);
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
        asm_printf("ERROR: set_chunk_size() got a new chunk size = %lu that is not a power of two\n", new_chunk_size);
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

/*******************/
/* PROCESS REQUEST */
/*******************/

void process_request(const uint64_t * request, uint64_t * response, bool * bReset, bool * bShutdown)
{
    // Initialize response with default values
    *bReset = false;
    *bShutdown = false;

    // Switch on request type
    switch (request[0])
    {
        case TYPE_PING:
        {
#ifdef DEBUG
            if (verbose) asm_printf("PING received\n");
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
            if (verbose) asm_printf("MINIMAL TRACE received\n");
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

                *bReset = true;
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
            if (verbose) asm_printf("ROM HISTOGRAM received\n");
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

                *bReset = true;
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
            if (verbose) asm_printf("MEMORY OPERATIONS received\n");
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

                *bReset = true;
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
            if (verbose) asm_printf("MAIN TRACE received\n");
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

                *bReset = true;
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
            if (verbose) asm_printf("COLLECT MEMORY received\n");
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

                *bReset = true;
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
            if (verbose) asm_printf("FAST received\n");
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

                *bReset = true;
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
            if (verbose) asm_printf("MEMORY READS received\n");
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

                *bReset = true;
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
            if (verbose) asm_printf("COLLECT MAIN received\n");
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

                *bReset = true;
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
            if (!silent) asm_printf("SHUTDOWN received\n");
            *bShutdown = true;

            response[0] = TYPE_SD_RESPONSE;
            response[1] = 0;
            response[2] = 0;
            response[3] = 0;
            response[4] = 0;
            break;
        }
        default:
        {
            asm_printf("ERROR: Invalid request id=%lu\n", request[0]);
            exit(-1);                
        }
    }
}

/**************/
/* TCP SERVER */
/**************/

void tcp_server (void)
{
    int result;

    // Create socket file descriptor
    int server_fd;
    server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (server_fd == 0)
    {
        asm_printf("ERROR: Failed calling socket() errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }

    // Forcefully attach socket to the port (avoid "address already in use")
    int opt = 1;
    result = setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR | SO_REUSEPORT, &opt, sizeof(opt));
    if (result != 0)
    {
        asm_printf("ERROR: Failed calling setsockopt() result=%d errno=%d=%s\n", result, errno, strerror(errno));
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
        asm_printf("ERROR: Failed calling bind() result=%d errno=%d=%s\n", result, errno, strerror(errno));
        exit(-1);
    }

    // Start listening
    result = listen(server_fd, 5);
    if (result != 0)
    {
        asm_printf("ERROR: Failed calling listen() result=%d errno=%d=%s\n", result, errno, strerror(errno));
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
            asm_printf("Waiting for incoming connections to port %u...\n", port);
        }
        client_fd = accept(server_fd, (struct sockaddr *)&address, (socklen_t*)&addrlen);
        if (client_fd < 0)
        {
            asm_printf("ERROR: Failed calling accept() client_fd=%d errno=%d=%s\n", client_fd, errno, strerror(errno));
            exit(-1);
        }
#ifdef DEBUG
        if (verbose) asm_printf("New client: %s:%d\n", inet_ntoa(address.sin_addr), ntohs(address.sin_port));
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
                asm_printf("ERROR: Failed calling recv() bytes_read=%ld errno=%d=%s\n", bytes_read, errno, strerror(errno));
                break;
            }
            if (bytes_read != sizeof(request))
            {
                if ((errno != 0) && (errno != 2))
                {
                    asm_printf("WARNING: Failed calling recv() invalid bytes_read=%ld errno=%d=%s\n", bytes_read, errno, strerror(errno));
                }
                break;
            }
#ifdef DEBUG
            if (verbose)
            {
                asm_printf("recv() returned: %ld\n", bytes_read);
            }
#endif
            if (verbose)
            {
                asm_printf("recv()'d request=[%lu, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n", request[0], request[1], request[2], request[3], request[4]);
            }

            // Process request and get response
            uint64_t response[5];
            process_request(request, response, &bReset, &bShutdown);

            // Send response to client
            if (verbose)
            {
                asm_printf("send()'ing response=[%lu, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n", response[0], response[1], response[2], response[3], response[4]);
            }
            // Send response to client, handling partial writes
            size_t total_size = sizeof(response);
            size_t total_sent = 0;
            while (total_sent < total_size)
            {
                ssize_t bytes_sent = send(client_fd,
                                          (const char *)response + total_sent,
                                          total_size - total_sent,
                                          0);
                if (bytes_sent < 0)
                {
                    if (errno == EINTR)
                    {
                        // Interrupted by signal, retry send
                        continue;
                    }
                    asm_printf("ERROR: Failed calling send() invalid bytes_sent=%zd errno=%d=%s\n",
                               bytes_sent, errno, strerror(errno));
                    break;
                }
                if (bytes_sent == 0)
                {
                    // Peer has performed an orderly shutdown
                    asm_printf("ERROR: Failed calling send(): connection closed by peer\n");
                    break;
                }
                total_sent += (size_t)bytes_sent;
            }
            if (total_sent != total_size)
            {
                asm_printf("ERROR: Failed calling send() invalid total_sent=%zu errno=%d=%s\n", total_sent, errno, strerror(errno));
                break;
            }
            else if (verbose)
            {
                asm_printf("Response sent to client\n");
            }

            // Reset the server if requested by the client
            if (bReset)
            {
                server_reset_slow();
            }

            // Shutdown if requested by the client
            if (bShutdown)
            {
                break;
            }
        }

        // Shutdown the client socket
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
}

/****************/
/* STDIO SERVER */
/****************/

void stdio_server (void)
{
    bool bShutdown = false;
    bool bReset;

    if (!silent)
    {
        asm_printf("Waiting for incoming data from stdin in pid=%d...\n", process_id);
    }

    // Disable buffering on stdin and stdout
    setbuf(stdin, NULL);
    setbuf(stdout, NULL);

    while (true)
    {
        // Read client request
        uint64_t request[5];
        size_t total_read = 0;
        bool read_error = false;
        while (total_read < sizeof(request))
        {
            ssize_t bytes_read = read(STDIN_FILENO,
                                      (char *)request + total_read,
                                      sizeof(request) - total_read);
            if (bytes_read < 0)
            {
                if (errno == EINTR)
                {
                    continue;
                }
                asm_printf("WARNING: Failed calling read(stdin) bytes_read=%zd errno=%d=%s\n", bytes_read, errno, strerror(errno));
                read_error = true;
                break;
            }
            if (bytes_read == 0)
            {
                // EOF before full request received
                read_error = true;
                break;
            }
            total_read += (size_t)bytes_read;
        }
#ifdef DEBUG
        if (verbose)
        {
            asm_printf("read(stdin) returned total_read: %zu\n", total_read);
        }
#endif
        if (read_error || (total_read != sizeof(request)))
        {
            break;
        }
        if (verbose)
        {
            asm_printf("read(stdin)'d request=[%lu, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n", request[0], request[1], request[2], request[3], request[4]);
        }

        // Process request and get response
        uint64_t response[5];
        process_request(request, response, &bReset, &bShutdown);

        if (verbose)
        {
            asm_printf("write(stdout)'ing response=[%lu, 0x%lx, 0x%lx, 0x%lx, 0x%lx]\n", response[0], response[1], response[2], response[3], response[4]);
        }

        // Write response to client
        size_t total_sent = 0;
        bool write_error = false;
        while (total_sent < sizeof(response))
        {
            ssize_t bytes_sent = write(STDOUT_FILENO,
                                       (const char *)response + total_sent,
                                       sizeof(response) - total_sent);
            if (bytes_sent < 0)
            {
                if (errno == EINTR)
                {
                    // Interrupted by signal, retry write
                    continue;
                }
                asm_printf("ERROR: Failed calling write(stdout) invalid bytes_sent=%zd errno=%d=%s\n", bytes_sent, errno, strerror(errno));
                write_error = true;
                break;
            }
            else if (bytes_sent == 0)
            {
                asm_printf("ERROR: write(stdout) returned 0, response not fully sent\n");
                break;
            }
            total_sent += (size_t)bytes_sent;
        }
        if (write_error || (total_sent != sizeof(response)))
        {
            asm_printf("ERROR: Failed calling write(stdout) invalid total_sent=%zu errno=%d=%s\n", total_sent, errno, strerror(errno));
            break;
        }
        else if (verbose)
        {
            asm_printf("Response sent to client\n");
        }

        // Reset the server if requested by the client
        if (bReset)
        {
            server_reset_slow();
        }

        // Shutdown if requested by the client
        if (bShutdown)
        {
            break;
        }
    }
}

/********/
/* MAIN */
/********/

int main(int argc, char *argv[])
{
#ifdef DEBUG
    // Start counting total execution time
    gettimeofday(&total_start_time, NULL);
#endif

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
        FILE * file_pointer;
        if (!stdio)
        {
            file_pointer = freopen(redirect_output_file, "w", stdout);
            if (file_pointer == NULL)
            {
                asm_printf("ERROR: Failed to redirect stdout to file %s\n", redirect_output_file);
                exit(-1);
            }
        }
        
        // Redirect stderr to the same file
        file_pointer = freopen(redirect_output_file, "a", stderr);
        if (file_pointer == NULL)
        {
            asm_printf("ERROR: Failed to redirect stderr to file %s\n", redirect_output_file);
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
        //
        // Client setup is deferred until after the initial ping to the server to map the shared
        // memories just created by the server in the stdio case; otherwise the client could either
        // not find any shared memory, or map old shared memories that the server will unlink during
        // its setup before creating new ones, causing the client to read and write to shared
        // memories that the server will not use anymore.
        // In the TCP case this is not an issue because the server creates the shared memories
        // before it starts listening to clients, so the client can map them during its setup even
        // before pinging the server, but we keep the procedure common to both cases.
        //
        // client_setup();

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

    // In case we just want to create the shared memories and exit, do it now after the setup and reset, and exit before starting to listen to clients
    if (just_create_all_shm)
    {
        server_cleanup();
        return 0;
    }

    // Run the proper server (stdio or TCP depending on configuration) to listen to client requests and process them
    if (stdio)
    {
        // Run the stdio server to listen to client requests and process them
        stdio_server();
    }
    else
    {
        // Run the TCP server to listen to client requests and process them
        tcp_server();
    }

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
        if (verbose) asm_printf("Emulator C end total_duration = %lu us assembly_duration = %lu us (%lu %%o)\n", total_duration, assembly_duration, assembly_percentage);
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
        asm_printf("ERROR: file_lock() failed calling open(%s) errno=%d=%s\n", file_lock_name, errno, strerror(errno));
        exit(1);
    }

    // Try to acquire an exclusive lock, non-blocking.
    if (flock(file_lock_fd, LOCK_EX | LOCK_NB) == -1) {
        // If we fail to get the lock, another instance is running.
        asm_printf("ERROR: Another instance of this program is already running.\n");
        exit(1);
    }
}

#endif // USE_FILE_LOCK