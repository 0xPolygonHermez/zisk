#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <errno.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <sys/mman.h>
#include <sys/time.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <netinet/tcp.h>
#include <semaphore.h>
#include <fcntl.h>
#include <unistd.h>
#include "constants.hpp"
#include "client.hpp"
#include "globals.hpp"
#include "emu.hpp"
#include "log.hpp"

void * shmem_input_address = NULL;

/*******/
/* TCP */
/*******/

int socket_fd = -1;

void client_tcp_connect ( void )
{
    // Create socket to connect to server
    socket_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (socket_fd < 0)
    {
        asm_printf("ERROR: socket() failed socket_fd=%d errno=%d=%s\n", socket_fd, errno, strerror(errno));
        exit(-1);
    }

    // Configure server address
    struct sockaddr_in server_addr;
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(port);
    
    int result = inet_pton(AF_INET, SERVER_IP, &server_addr.sin_addr);
    if (result <= 0)
    {
        asm_printf("ERROR: inet_pton() failed.  Invalid address/Address not supported result=%d errno=%d=%s\n", result, errno, strerror(errno));
        exit(-1);
    }

    // Connect to server
    result = connect(socket_fd, (struct sockaddr *)&server_addr, sizeof(server_addr));
    if (result < 0)
    {
        asm_printf("ERROR: connect() failed result=%d errno=%d=%s\n", result, errno, strerror(errno));
        exit(-1);
    }
    if (verbose) asm_printf("connect()'d to port=%u\n", port);
}

void client_tcp_close ( void )
{
    // Close the socket
    close(socket_fd);
}

void client_tcp_send ( const uint64_t * request )
{
    // Send data to server, handling partial writes and EINTR
    const uint8_t *buffer = (const uint8_t *)request;
    size_t bytes_to_send = 5 * sizeof(uint64_t);
    while (bytes_to_send > 0)
    {
        ssize_t result = send(socket_fd, buffer, bytes_to_send, 0);
        if (result < 0)
        {
            if (errno == EINTR)
            {
                // Interrupted by signal, retry send
                continue;
            }
            asm_printf("ERROR: send() failed result=%zd errno=%d=%s\n", result, errno, strerror(errno));
            exit(-1);
        }
        if (result == 0)
        {
            // Unexpected: send() returned 0 without sending data
            asm_printf("ERROR: send() returned 0 bytes errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }
        buffer += (size_t)result;
        bytes_to_send -= (size_t)result;
    }
}

void client_tcp_recv ( uint64_t * response )
{
    // Read server response, handling partial reads and EINTR
    uint8_t *buffer = (uint8_t *)response;
    size_t bytes_remaining = 5 * sizeof(uint64_t);
    while (bytes_remaining > 0)
    {
        ssize_t result = recv(socket_fd, buffer, bytes_remaining, 0);
        if (result < 0)
        {
            if (errno == EINTR)
            {
                // Interrupted by signal, retry recv
                continue;
            }
            asm_printf("ERROR: recv() failed result=%zd errno=%d=%s\n", result, errno, strerror(errno));
            exit(-1);
        }
        if (result == 0)
        {
            // Peer closed connection unexpectedly
            asm_printf("ERROR: recv() returned 0 bytes (peer closed connection) errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }
        buffer += (size_t)result;
        bytes_remaining -= (size_t)result;
    }
}

/*********/
/* STDIO */
/*********/

int server_stdin_fd = -1;
int server_stdout_fd = -1;
char server_stdin_name[256];
char server_stdout_name[256];

void client_stdio_connect ( void )
{
    // Set paths based on server PID or default to fifos if server PID is not provided
    if (server_pid > 0)
    {
        // Construct paths to server's stdin and stdout based on server PID, e.g., /proc/12345/fd/0 and /proc/12345/fd/1
        snprintf(server_stdin_name, sizeof(server_stdin_name), "/proc/%d/fd/0", server_pid);
        snprintf(server_stdout_name, sizeof(server_stdout_name), "/proc/%d/fd/1", server_pid);
    }
    else
    {
        // Default to fifos with fixed names if server PID is not provided (e.g., for testing with a manually launched server)
        // Precondition 1: These fifos must be created before launching the client, e.g., with "mkfifo /tmp/fifoinput /tmp/fifooutput"
        // Precondition 2: The server must be launched with its stdout and stdin redirected to these fifos, e.g.
        // emulator-asm/build/ziskemuasm -s --gen=1 -v -m --stdio --redirect-output-to-file < /tmp/fifoinput > /tmp/fifooutput &
        // The server process will not exist until the client opens the fifos, so the client must be launched after the server in this state
        // The server process will stop existing after the client closes the fifos, i.e. after client completion
        sprintf(server_stdin_name, "/tmp/fifoinput");
        sprintf(server_stdout_name, "/tmp/fifooutput");
    }
    
    // Open server's stdin (we write to it)
    server_stdin_fd = open(server_stdin_name, O_WRONLY);
    if (server_stdin_fd < 0)
    {
        asm_printf("ERROR: Failed opening open(%s) errno=%d=%s\n", server_stdin_name, errno, strerror(errno));
        exit(-1);
    }
    if (verbose) asm_printf("Opened %s for writing\n", server_stdin_name);

    // Open server's stdout (we read from it)
    server_stdout_fd = open(server_stdout_name, O_RDONLY);
    if (server_stdout_fd < 0)
    {
        asm_printf("ERROR: Failed opening open(%s) errno=%d=%s\n", server_stdout_name, errno, strerror(errno));
        exit(-1);
    }
    if (verbose) asm_printf("Opened %s for reading\n", server_stdout_name);
}

void client_stdio_close ( void )
{
    if (server_stdout_fd >= 0) close(server_stdout_fd);
    if (server_stdin_fd >= 0) close(server_stdin_fd);
}

void client_stdio_send ( const uint64_t * request )
{
    size_t bytes_to_write = 5 * sizeof(uint64_t);
    size_t total_written = 0;
    while (total_written < bytes_to_write)
    {
        ssize_t bytes_written = write(server_stdin_fd,
                                      ((const char *)request) + total_written,
                                      bytes_to_write - total_written);
        if (bytes_written < 0)
        {
            if (errno == EINTR)
            {
                continue;  // Interrupted, retry
            }
            asm_printf("ERROR: Failed calling write() bytes_written=%zd errno=%d=%s\n",
                       bytes_written, errno, strerror(errno));
            exit(-1);
        }
        if (bytes_written == 0)
        {
            asm_printf("ERROR: write() returned 0 bytes, unexpected EOF on %s\n", server_stdin_name);
            exit(-1);
        }
        total_written += (size_t)bytes_written;
    }
    if (verbose) asm_printf("Wrote %zu bytes to %s\n", total_written, server_stdin_name);
}

void client_stdio_recv ( uint64_t * response )
{
    size_t bytes_to_read = 5 * sizeof(uint64_t);
    ssize_t total_read = 0;
    
    while (total_read < bytes_to_read)
    {
        ssize_t bytes_read = read(server_stdout_fd, ((char*)response) + total_read, bytes_to_read - total_read);
        if (bytes_read < 0)
        {
            if (errno == EINTR)
            {
                continue;  // Interrupted, retry
            }
            asm_printf("ERROR: Failed calling read() bytes_read=%zd errno=%d=%s\n", bytes_read, errno, strerror(errno));
            exit(-1);
        }
        if (bytes_read == 0)
        {
            asm_printf("ERROR: Unexpected EOF while reading response\n");
            exit(-1);
        }
        total_read += bytes_read;
    }
    if (verbose) asm_printf("Read %zd bytes from %s, total_read=%zd\n", total_read, server_stdout_name, total_read);
}

/******/
/* IO */
/******/

void client_io_connect ( void )
{
    if (stdio)
    {
        client_stdio_connect();
    }
    else
    {
        client_tcp_connect();
    }
}

void client_io_close ( void )
{
    if (stdio)
    {
        client_stdio_close();
    }
    else
    {
        client_tcp_close();
    }
}

void client_io_send ( const uint64_t * request )
{
    if (stdio)
    {
        client_stdio_send(request);
    }
    else
    {
        client_tcp_send(request);
    }
}

void client_io_recv ( uint64_t * response )
{
    if (stdio)
    {
        client_stdio_recv(response);
    }
    else
    {
        client_tcp_recv(response);
    }
}

/*********/
/* SETUP */
/*********/

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
            asm_printf("ERROR: Failed calling trace shm_open(%s) errno=%d=%s\n", shmem_mt_name, errno, strerror(errno));
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
            asm_printf("ERROR: Failed calling mmap(MT) errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }
        if ((uint64_t)pTrace != TRACE_ADDR)
        {
            asm_printf("ERROR: Called mmap(MT) but returned address = %p != 0x%lx\n", pTrace, TRACE_ADDR);
            exit(-1);
        }
        if (verbose) asm_printf("mmap(MT) returned %p in %lu us\n", pTrace, duration);
    }

    /**********************/
    /* PRECOMPILE_RESULTS */
    /**********************/

    if (precompile_results_enabled)
    {
        /**************/
        /* PRECOMPILE */
        /**************/

        // Create the precompile results shared memory
        shmem_precompile_fd = shm_open(shmem_precompile_name, O_RDWR, 0666);
        if (shmem_precompile_fd < 0)
        {
            asm_printf("ERROR: Failed calling precompile shm_open(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
            exit(-1);
        }

        // Map precompile address space
        if (verbose) gettimeofday(&start_time, NULL);
        void * pPrecompile = mmap(NULL, MAX_PRECOMPILE_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | map_locked_flag, shmem_precompile_fd, 0);
        if (verbose)
        {
            gettimeofday(&stop_time, NULL);
            duration = TimeDiff(start_time, stop_time);
        }
        if (pPrecompile == MAP_FAILED)
        {
            asm_printf("ERROR: Failed calling mmap(precompile) errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }
        shmem_precompile_address = pPrecompile;
        precompile_results_address = (uint64_t *)pPrecompile;

        if (verbose) asm_printf("mmap(precompile) mapped %lu B and returned address %p in %lu us\n", MAX_PRECOMPILE_SIZE, precompile_results_address, duration);

        /*************************/
        /* PRECOMPILE SEMAPHORES */
        /*************************/

        // Create the semaphore for precompile results available signal
        assert(strlen(sem_prec_avail_name) > 0);

        sem_prec_avail = sem_open(sem_prec_avail_name, O_CREAT, 0666, 0);
        if (sem_prec_avail == SEM_FAILED)
        {
            asm_printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_prec_avail_name, errno, strerror(errno));
            exit(-1);
        }
        if (verbose) asm_printf("sem_open(%s) succeeded\n", sem_prec_avail_name);

        // Create the semaphore for precompile results read signal
        assert(strlen(sem_prec_read_name) > 0);

        sem_prec_read = sem_open(sem_prec_read_name, O_CREAT, 0666, 0);
        if (sem_prec_read == SEM_FAILED)
        {
            asm_printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_prec_read_name, errno, strerror(errno));
            exit(-1);
        }
        if (verbose) asm_printf("sem_open(%s) succeeded\n", sem_prec_read_name);
    }

    /*****************/
    /* CONTROL INPUT */
    /*****************/

    // Create the control input shared memory
    shmem_control_input_fd = shm_open(shmem_control_input_name, O_RDWR, 0666);
    if (shmem_control_input_fd < 0)
    {
        asm_printf("ERROR: Failed calling control shm_open(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
        exit(-1);
    }

    // Map control input address space
    if (verbose) gettimeofday(&start_time, NULL);
    void * pControl = mmap((void *)CONTROL_INPUT_ADDR, CONTROL_INPUT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_control_input_fd, 0);
    if (verbose)
    {
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
    }
    if (pControl == MAP_FAILED)
    {
        asm_printf("ERROR: Failed calling mmap(control_input) errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }
    if (pControl != (void *)CONTROL_INPUT_ADDR)
    {
        asm_printf("ERROR: Called mmap(control_input) but returned address = %p != 0x%08lx\n", pControl, CONTROL_INPUT_ADDR);
        exit(-1);
    }
    shmem_control_input_address = (uint64_t *)pControl;
    precompile_written_address = &shmem_control_input_address[0];
    precompile_exit_address = &shmem_control_input_address[1];
    input_written_address = &shmem_control_input_address[2];
    if (verbose) asm_printf("mmap(control_input) mapped %lu B and returned address %p in %lu us\n", CONTROL_INPUT_SIZE, shmem_control_input_address, duration);

    /*****************/
    /* CONTROL OUTPUT */
    /*****************/

    // Create the control input shared memory
    shmem_control_output_fd = shm_open(shmem_control_output_name, O_RDWR, 0666);
    if (shmem_control_output_fd < 0)
    {
        asm_printf("ERROR: Failed calling control shm_open(%s) errno=%d=%s\n", shmem_control_output_name, errno, strerror(errno));
        exit(-1);
    }

    // Map control input address space
    if (verbose) gettimeofday(&start_time, NULL);
    pControl = mmap((void *)CONTROL_OUTPUT_ADDR, CONTROL_OUTPUT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED | map_locked_flag, shmem_control_output_fd, 0);
    if (verbose)
    {
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
    }
    if (pControl == MAP_FAILED)
    {
        asm_printf("ERROR: Failed calling mmap(control_output) errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }
    if (pControl != (void *)CONTROL_OUTPUT_ADDR)
    {
        asm_printf("ERROR: Called mmap(control_output) but returned address = %p != 0x%08lx\n", pControl, CONTROL_OUTPUT_ADDR);
        exit(-1);
    }
    shmem_control_output_address = (uint64_t *)pControl;
    precompile_read_address = &shmem_control_output_address[0];
    if (verbose) asm_printf("mmap(control_output) mapped %lu B and returned address %p in %lu us\n", CONTROL_OUTPUT_SIZE, shmem_control_output_address, duration);
}

typedef enum {
    PrecompileReadMode_NoPrefix,
    PrecompileReadMode_Prefixed
} PrecompileReadMode;

PrecompileReadMode precompile_read_mode = PrecompileReadMode_NoPrefix;
//PrecompileReadMode precompile_read_mode = PrecompileReadMode_Prefixed;

typedef enum {
    PrecompileWriteMode_Full,
    PrecompileWriteMode_OnePrecAtATime
} PrecompileWriteMode;

PrecompileWriteMode precompile_write_mode = PrecompileWriteMode_Full;
//PrecompileWriteMode precompile_write_mode = PrecompileWriteMode_OnePrecAtATime;

//#define PRECOMPILE_FIXED_SIZE 25 // Keccak-f state size in u64s
#define PRECOMPILE_FIXED_SIZE 4 // SHA-256 state size in u64s

void client_write_precompile_results (void)
{
    int result;

#ifdef DEBUG
    gettimeofday(&start_time, NULL);
#endif

    // Open input file
    FILE * precompile_fp = fopen(precompile_file_name, "r");
    if (precompile_fp == NULL)
    {
        asm_printf("ERROR: Failed calling fopen(%s) errno=%d=%s; does it exist?\n", precompile_file_name, errno, strerror(errno));
        exit(-1);
    }

    // Get input file size
    if (fseek(precompile_fp, 0, SEEK_END) == -1)
    {
        asm_printf("ERROR: Failed calling fseek(%s) errno=%d=%s\n", precompile_file_name, errno, strerror(errno));
        exit(-1);
    }
    long precompile_data_size = ftell(precompile_fp);
    if (precompile_data_size == -1)
    {
        asm_printf("ERROR: Failed calling ftell(%s) errno=%d=%s\n", precompile_file_name, errno, strerror(errno));
        exit(-1);
    }
    if ((precompile_data_size & 0x7) != 0)
    {
        asm_printf("ERROR: Precompile results file (%s) size (%ld) is not a multiple of 8 B\n", precompile_file_name, precompile_data_size);
        exit(-1);
    }

    // Go back to the first byte
    if (fseek(precompile_fp, 0, SEEK_SET) == -1)
    {
        asm_printf("ERROR: Failed calling fseek(%s, 0) errno=%d=%s\n", precompile_file_name, errno, strerror(errno));
        exit(-1);
    }

    assert(precompile_read_mode == PrecompileReadMode_NoPrefix || precompile_read_mode == PrecompileReadMode_Prefixed);
    assert(precompile_write_mode == PrecompileWriteMode_Full || precompile_write_mode == PrecompileWriteMode_OnePrecAtATime);

    /*************/
    /* NO PREFIX */
    /*************/

    if (precompile_read_mode == PrecompileReadMode_NoPrefix)
    {
        if (precompile_write_mode == PrecompileWriteMode_Full)
        {
            // Check the precompile data size is inside the proper range
            if (precompile_data_size > MAX_PRECOMPILE_SIZE)
            {
                asm_printf("ERROR: Size of precompile results file (%s) is too long (%ld)\n", precompile_file_name, precompile_data_size);
                exit(-1);
            }

            // Copy input data into input memory
            size_t precompile_read = fread(precompile_results_address, 1, precompile_data_size, precompile_fp);
            if (precompile_read != precompile_data_size)
            {
                asm_printf("ERROR: Input read (%zu) != expected read size (%ld)\n", precompile_read, precompile_data_size);
                exit(-1);
            }

            // Initialize precompile written address
            *precompile_written_address = precompile_data_size >> 3; // in u64s

            //asm_printf("Posting sem_prec_avail() precompile_written=%lu precompile_read=%lu\n", *precompile_written_address, *precompile_read_address);
            sem_post(sem_prec_avail);
        }
        else if (precompile_write_mode == PrecompileWriteMode_OnePrecAtATime)
        {
            // Check the precompile data size is inside the proper range
            if (precompile_data_size % (PRECOMPILE_FIXED_SIZE * 8) != 0)
            {
                asm_printf("ERROR: Size of precompile results file (%s) is not a multiple %u * 8 B\n", precompile_file_name, PRECOMPILE_FIXED_SIZE);
                exit(-1);
            }

            // Initialize precompile written address to zero
            *precompile_written_address = 0; // in u64s

            // Copy in chunks of PRECOMPILE_FIXED_SIZE*8 bytes (Keccak-f state size)
            uint64_t precompile_read_so_far = 0;
            uint64_t data[PRECOMPILE_FIXED_SIZE];
            while (precompile_read_so_far < (uint64_t)precompile_data_size)
            {        
                // Wait for server to read precompile results
                //asm_printf("Waiting for sem_prec_read()\n");
                result = sem_wait(sem_prec_read);
                if (result == -1)
                {
                    asm_printf("ERROR: Failed calling sem_wait(sem_prec_read) errno=%d=%s\n", errno, strerror(errno));
                    exit(-1);
                }

                // Number of bytes to read from file and write to shared memory in every loop
                uint64_t bytes_to_read = sizeof(data);

                // Copy input data into input memory
                size_t precompile_read = fread(data, 1, bytes_to_read, precompile_fp);
                if (precompile_read != bytes_to_read)
                {
                    asm_printf("ERROR: Input read (%zu) != expected read size (%zu)\n", precompile_read, bytes_to_read);
                    exit(-1);
                }

                // Copy data to shared memory
                for (int i=0; i<PRECOMPILE_FIXED_SIZE; i++)
                {
                    memcpy(&precompile_results_address[(precompile_read_so_far >> 3) % (MAX_PRECOMPILE_SIZE >> 3)], &data[i], 8);
                    precompile_read_so_far += 8;
                }

                // Notify server that precompile results are available
                *precompile_written_address = precompile_read_so_far >> 3; // in u64s

                //asm_printf("Posting sem_prec_avail() precompile_written=%lu precompile_read=%lu\n", *precompile_written_address, *precompile_read_address);
                sem_post(sem_prec_avail);
            }
        }
    }

    /************/
    /* PREFIXED */
    /************/

    else if (precompile_read_mode == PrecompileReadMode_Prefixed)
    {
#define CTRL_START 0x00
#define CTRL_END 0x01
#define CTRL_CANCEL 0x02
#define CTRL_ERROR 0x03
#define HINTS_TYPE_RESULT 0x04
#define HINTS_TYPE_ECRECOVER 0x05
#define NUM_HINT_TYPES 0x06

        uint64_t precompile_read_so_far = 0;
        uint64_t precompile_written_so_far = 0;

        while (precompile_read_so_far < (uint64_t)precompile_data_size)
        {
            uint64_t data;
            size_t bytes_to_read = sizeof(data);

            // Copy input data into input memory
            size_t precompile_read = fread(&data, 1, bytes_to_read, precompile_fp);
            if (precompile_read != bytes_to_read)
            {
                asm_printf("ERROR: Input read (%zu) != expected read size (%zu)\n", precompile_read, bytes_to_read);
                exit(-1);
            }
            precompile_read_so_far += bytes_to_read;
            switch (data >> 32)
            {
                case CTRL_START:
                    //asm_printf("Precompile CTRL_START\n");
                    assert(precompile_read_so_far == 8);
                    break;
                case CTRL_END:
                    //asm_printf("Precompile CTRL_END\n");
                    assert(precompile_read_so_far == precompile_data_size);
                    break;
                // case CTRL_CANCEL:
                //     asm_printf("Precompile CTRL_CANCEL\n");
                //     break;
                // case CTRL_ERROR:
                //     asm_printf("Precompile CTRL_ERROR\n");
                //     break;
                case HINTS_TYPE_RESULT:
                {
                    //asm_printf("Precompile HINTS_TYPE_RESULT\n");
                    if (precompile_write_mode == PrecompileWriteMode_OnePrecAtATime)
                    {
                        // Wait for server to read precompile results
                        //asm_printf("Waiting for sem_prec_read()\n");
                        result = sem_wait(sem_prec_read);
                        if (result == -1)
                        {
                            asm_printf("ERROR: Failed calling sem_wait(sem_prec_read) errno=%d=%s\n", errno, strerror(errno));
                            exit(-1);
                        }
                    }

                    uint64_t result_length = data & 0xFFFFFFFF;
                    if (result_length > (precompile_data_size - precompile_read_so_far))
                    {
                        asm_printf("ERROR: Precompile HINTS_TYPE_RESULT length=%lu exceeds remaining file size %lu\n", result_length, precompile_data_size - precompile_read_so_far);
                        exit(-1);
                    }
                    //asm_printf("Precompile HINTS_TYPE_RESULT result_length=%lu\n", result_length);
                    for (uint64_t i=0; i<result_length; i++)
                    {
                        uint64_t value;
                        size_t precompile_read = fread(&value, 1, 8, precompile_fp);
                        if (precompile_read != 8)
                        {
                            asm_printf("ERROR: Input read (%zu) != expected read size (8)\n", precompile_read);
                            exit(-1);
                        }
                        memcpy(&precompile_results_address[(precompile_written_so_far >> 3) % (MAX_PRECOMPILE_SIZE >> 3)], &value, 8);
                        precompile_read_so_far += 8;
                        precompile_written_so_far += 8;
                        //asm_printf("  Precompile result[%lu] = 0x%016lx\n", i, value);
                    }

                    if (precompile_write_mode == PrecompileWriteMode_OnePrecAtATime)
                    {
                        // Notify server that precompile results are available
                        *precompile_written_address = precompile_written_so_far >> 3; // in u64s

                        //asm_printf("Posting sem_prec_avail() precompile_written=%lu precompile_read=%lu\n", *precompile_written_address, *precompile_read_address);
                        sem_post(sem_prec_avail);
                    }
                }
                break;
                // case HINTS_TYPE_ECRECOVER:
                //     {
                //         // Not implemented
                //         asm_printf("Precompile HINTS_TYPE_ECRECOVER not implemented\n");
                //     }
                //     break;
                default:
                    asm_printf("ERROR: Unknown precompile prefix type %lu\n", data >> 32);
                    exit(-1);
            }
        }

        if (precompile_write_mode == PrecompileWriteMode_Full)
        {
            // Notify server that precompile results are available
            *precompile_written_address = precompile_written_so_far >> 3; // in u64s

            //asm_printf("Posting sem_prec_avail() precompile_written=%lu precompile_read=%lu\n", *precompile_written_address, *precompile_read_address);
            sem_post(sem_prec_avail);
        }

    }

    // Close the file pointer
    fclose(precompile_fp);

#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    asm_printf("client (precompile): done in %lu us\n", duration);
#endif
}

void client_run (void)
{
    asm_printf("client_run(): Starting client...\n");
    assert(client);
    assert(!server);

    int result;

    /*************************/
    /* Connect to the server */
    /*************************/
    client_io_connect();

    // Request and response, to be used to communicate with the server.
    // The first 64 bits of the request is the type, and the rest are arguments.
    // The response format depends on the request type.
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
    client_io_send(request);

    // Read server response
    client_io_recv(response);
    
    if (response[0] != TYPE_PONG)
    {
        asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
        exit(-1);
    }
    if (response[1] != gen_method)
    {
        asm_printf("ERROR: recv() returned unexpected gen_method=%lu\n", response[1]);
        exit(-1);
    }

    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    asm_printf("client (PING): done in %lu us\n", duration);

    client_setup();

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
            asm_printf("ERROR: Failed calling fopen(%s) errno=%d=%s; does it exist?\n", input_file, errno, strerror(errno));
            exit(-1);
        }

        // Get input file size
        if (fseek(input_fp, 0, SEEK_END) == -1)
        {
            asm_printf("ERROR: Failed calling fseek(%s) errno=%d=%s\n", input_file, errno, strerror(errno));
            exit(-1);
        }
        long input_data_size = ftell(input_fp);
        if (input_data_size == -1)
        {
            asm_printf("ERROR: Failed calling ftell(%s) errno=%d=%s\n", input_file, errno, strerror(errno));
            exit(-1);
        }

        // Go back to the first byte
        if (fseek(input_fp, 0, SEEK_SET) == -1)
        {
            asm_printf("ERROR: Failed calling fseek(%s, 0) errno=%d=%s\n", input_file, errno, strerror(errno));
            exit(-1);
        }

        // Check the input data size is inside the proper range
        if (input_data_size > (MAX_INPUT_SIZE - 16))
        {
            asm_printf("ERROR: Size of input file (%s) is too long (%ld)\n", input_file, input_data_size);
            exit(-1);
        }

        // Open input shared memory
        shmem_input_fd = shm_open(shmem_input_name, O_RDWR, 0666);
        if (shmem_input_fd < 0)
        {
            asm_printf("ERROR: Failed calling input shm_open(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            exit(-1);
        }

        // Map the shared memory object into the process address space
        shmem_input_address = mmap(NULL, MAX_INPUT_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, shmem_input_fd, 0);
        if (shmem_input_address == MAP_FAILED)
        {
            asm_printf("ERROR: Failed calling mmap(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            exit(-1);
        }

        // Write the free input value as 0 in the first 64 bits
        *(uint64_t *)shmem_input_address = (uint64_t)0; // free input

        // Copy input data into input memory
        size_t input_read = fread(shmem_input_address + 8, 1, input_data_size, input_fp);
        if (input_read != input_data_size)
        {
            asm_printf("ERROR: Input read (%zu) != input file size (%ld)\n", input_read, input_data_size);
            exit(-1);
        }

        // Close the file pointer
        fclose(input_fp);

        // Unmap input
        result = munmap(shmem_input_address, MAX_INPUT_SIZE);
        if (result == -1)
        {
            asm_printf("ERROR: Failed calling munmap(input) errno=%d=%s\n", errno, strerror(errno));
            exit(-1);
        }

        // Set written counter
        *input_written_address = input_data_size; // in bytes

#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        asm_printf("client (input): done in %lu us\n", duration);
#endif

    }

    /*****************************/
    /* Read precompile file data */
    /*****************************/
    if (precompile_results_enabled)
    {
        // Reset written counter
        *precompile_written_address = 0;

        //client_write_precompile_results();
    }

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
                request[1] = MAX_STEPS;
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                if (precompile_results_enabled)
                {
                    client_write_precompile_results();
                }

                // Send data to server
                client_io_send(request);

                // Read server response
                client_io_recv(response);

                if (response[0] != TYPE_MT_RESPONSE)
                {
                    asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                asm_printf("client (MT)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));

                break;
            }
            case RomHistogram:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_RH_REQUEST;
                request[1] = MAX_STEPS;
                request[2] = 0;
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                client_io_send(request);

                if (precompile_results_enabled)
                {
                    client_write_precompile_results();
                }

                // Read server response
                client_io_recv(response);
                
                if (response[0] != TYPE_RH_RESPONSE)
                {
                    asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                asm_printf("client (RH)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));

                break;
            }
            case MemOp:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_MO_REQUEST;
                request[1] = MAX_STEPS;
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                client_io_send(request);

                if (precompile_results_enabled)
                {
                    client_write_precompile_results();
                }

                // Read server response
                client_io_recv(response);
                
                if (response[0] != TYPE_MO_RESPONSE)
                {
                    asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                asm_printf("client (MO)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));
                
                break;
            }
            case MainTrace:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_MA_REQUEST;
                request[1] = MAX_STEPS;
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                client_io_send(request);

                // Read server response
                client_io_recv(response);
                
                if (response[0] != TYPE_MA_RESPONSE)
                {
                    asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                asm_printf("client (MA)[%lu]: done in %lu us\n", i, duration);

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
                    request[1] = MAX_STEPS;
                    request[2] = 1ULL << 18; // chunk_len
                    request[3] = chunk_player_address;
                    request[4] = 0;

                    // Send data to server
                    client_io_send(request);

                    // Read server response
                    client_io_recv(response);
                    
                    if (response[0] != TYPE_CM_RESPONSE)
                    {
                        asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                        exit(-1);
                    }
                    if (response[1] != 0)
                    {
                        asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                        exit(-1);
                    }
                    
                    gettimeofday(&stop_time, NULL);
                    duration = TimeDiff(start_time, stop_time);
                    asm_printf("client (CM)[%lu]: done in %lu us\n", i, duration);
                }
                else
                {
                    uint64_t number_of_chunks = pInputTrace[4];
                    asm_printf("client (CM)[%lu]: sending requests for %lu chunks\n", i, number_of_chunks);

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

                        asm_printf("client (CM)[%lu][%lu]: @=0x%lx sending request...", i, c, chunk_player_address);

                        gettimeofday(&start_time, NULL);
    
                        // Prepare message to send
                        request[0] = TYPE_CM_REQUEST;
                        request[1] = MAX_STEPS;
                        request[2] = 1ULL << 18; // chunk_len
                        request[3] = chunk_player_address;
                        request[4] = 0;
    
                        // Send data to server
                        client_io_send(request);
    
                        // Read server response
                        client_io_recv(response);
                        
                        if (response[0] != TYPE_CM_RESPONSE)
                        {
                            asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                            exit(-1);
                        }
                        if (response[1] != 0)
                        {
                            asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                            exit(-1);
                        }
                        
                        gettimeofday(&stop_time, NULL);
                        duration = TimeDiff(start_time, stop_time);
                        asm_printf("done in %lu us\n", duration);
                    }

                } 
                
                break;
            }
            case Fast:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_FA_REQUEST;
                request[1] = MAX_STEPS;
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                client_io_send(request);

                // Read server response
                client_io_recv(response);
                
                if (response[0] != TYPE_FA_RESPONSE)
                {
                    asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                asm_printf("client (FA)[%lu]: done in %lu us\n", i, duration);

                // Pretend to spend some time processing the incoming data
                usleep((1000000));
                
                break;
            }
            case MemReads:
            {
                gettimeofday(&start_time, NULL);

                // Prepare message to send
                request[0] = TYPE_MR_REQUEST;
                request[1] = MAX_STEPS;
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                // Send data to server
                client_io_send(request);

                // Read server response
                client_io_recv(response);
                
                if (response[0] != TYPE_MR_RESPONSE)
                {
                    asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                    exit(-1);
                }
                if (response[1] != 0)
                {
                    asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                    exit(-1);
                }
                
                gettimeofday(&stop_time, NULL);
                duration = TimeDiff(start_time, stop_time);
                asm_printf("client (MR)[%lu]: done in %lu us\n", i, duration);

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
                    request[1] = MAX_STEPS;
                    request[2] = 1ULL << 18; // chunk_len
                    request[3] = chunk_player_address;
                    request[4] = 0;

                    // Send data to server
                    client_io_send(request);

                    // Read server response
                    client_io_recv(response);
                    
                    if (response[0] != TYPE_CA_RESPONSE)
                    {
                        asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                        exit(-1);
                    }
                    if (response[1] != 0)
                    {
                        asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                        exit(-1);
                    }
                    
                    gettimeofday(&stop_time, NULL);
                    duration = TimeDiff(start_time, stop_time);
                    asm_printf("client (CA)[%lu]: done in %lu us\n", i, duration);
                }
                else
                {
                    uint64_t number_of_chunks = pInputTrace[4];
                    asm_printf("client (CA)[%lu]: sending requests for %lu chunks\n", i, number_of_chunks);

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

                        asm_printf("client (CA)[%lu][%lu]: @=0x%lx sending request...", i, c, chunk_player_address);

                        gettimeofday(&start_time, NULL);
    
                        // Prepare message to send
                        request[0] = TYPE_CA_REQUEST;
                        request[1] = MAX_STEPS;
                        request[2] = 1ULL << 18; // chunk_len
                        request[3] = chunk_player_address;
                        request[4] = 0;
    
                        // Send data to server
                        client_io_send(request);
    
                        // Read server response
                        client_io_recv(response);
                        
                        if (response[0] != TYPE_CA_RESPONSE)
                        {
                            asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
                            exit(-1);
                        }
                        if (response[1] != 0)
                        {
                            asm_printf("ERROR: recv() returned unexpected result=%lu\n", response[1]);
                            exit(-1);
                        }
                        
                        gettimeofday(&stop_time, NULL);
                        duration = TimeDiff(start_time, stop_time);
                        asm_printf("done in %lu us\n", duration);
                    }

                } 
                
                break;
            }
            default:
            {
                asm_printf("client_run() found invalid gen_method=%d\n", gen_method);
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
    client_io_send(request);

    // Read server response
    client_io_recv(response);
    
    if (response[0] != TYPE_SD_RESPONSE)
    {
        asm_printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
        exit(-1);
    }
    
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    asm_printf("client (SD): done in %lu us\n", duration);

    } // do_shutdown

    /***********/
    /* Cleanup */
    /***********/

    // Close the socket
    client_io_close();
}

void client_cleanup (void)
{
    // Cleanup trace
    int result = munmap((void *)TRACE_ADDR, trace_size);
    if (result == -1)
    {
        asm_printf("ERROR: Failed calling munmap(trace) for size=%lu errno=%d=%s\n", trace_size, errno, strerror(errno));
    }
}