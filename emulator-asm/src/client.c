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

void * shmem_input_address = NULL;

/**********/
/* CLIENT */
/**********/

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
            printf("ERROR: Failed calling trace shm_open(%s) errno=%d=%s\n", shmem_mt_name, errno, strerror(errno));
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
            printf("ERROR: Failed calling precompile shm_open(%s) errno=%d=%s\n", shmem_precompile_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
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
            printf("ERROR: Failed calling mmap(precompile) errno=%d=%s\n", errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        shmem_precompile_address = pPrecompile;
        precompile_results_address = (uint64_t *)pPrecompile;

        if (verbose) printf("mmap(precompile) mapped %lu B and returned address %p in %lu us\n", MAX_PRECOMPILE_SIZE, precompile_results_address, duration);

        /*************************/
        /* PRECOMPILE SEMAPHORES */
        /*************************/

        // Create the semaphore for precompile results available signal
        assert(strlen(sem_prec_avail_name) > 0);

        sem_prec_avail = sem_open(sem_prec_avail_name, O_CREAT, 0666, 0);
        if (sem_prec_avail == SEM_FAILED)
        {
            printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_prec_avail_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("sem_open(%s) succeeded\n", sem_prec_avail_name);

        // Create the semaphore for precompile results read signal
        assert(strlen(sem_prec_read_name) > 0);

        sem_prec_read = sem_open(sem_prec_read_name, O_CREAT, 0666, 0);
        if (sem_prec_read == SEM_FAILED)
        {
            printf("ERROR: Failed calling sem_open(%s) errno=%d=%s\n", sem_prec_read_name, errno, strerror(errno));
            fflush(stdout);
            fflush(stderr);
            exit(-1);
        }
        if (verbose) printf("sem_open(%s) succeeded\n", sem_prec_read_name);
    }

    /*****************/
    /* CONTROL INPUT */
    /*****************/

    // Create the control input shared memory
    shmem_control_input_fd = shm_open(shmem_control_input_name, O_RDWR, 0666);
    if (shmem_control_input_fd < 0)
    {
        printf("ERROR: Failed calling control shm_open(%s) errno=%d=%s\n", shmem_control_input_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
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
        printf("ERROR: Failed calling mmap(control_input) errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (pControl != (void *)CONTROL_INPUT_ADDR)
    {
        printf("ERROR: Called mmap(control_input) but returned address = %p != 0x%08lx\n", pControl, CONTROL_INPUT_ADDR);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    shmem_control_input_address = (uint64_t *)pControl;
    precompile_written_address = &shmem_control_input_address[0];
    precompile_exit_address = &shmem_control_input_address[1];
    input_written_address = &shmem_control_input_address[2];
    if (verbose) printf("mmap(control_input) mapped %lu B and returned address %p in %lu us\n", CONTROL_INPUT_SIZE, shmem_control_input_address, duration);

    /*****************/
    /* CONTROL OUTPUT */
    /*****************/

    // Create the control input shared memory
    shmem_control_output_fd = shm_open(shmem_control_output_name, O_RDWR, 0666);
    if (shmem_control_output_fd < 0)
    {
        printf("ERROR: Failed calling control shm_open(%s) errno=%d=%s\n", shmem_control_output_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
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
        printf("ERROR: Failed calling mmap(control_output) errno=%d=%s\n", errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (pControl != (void *)CONTROL_OUTPUT_ADDR)
    {
        printf("ERROR: Called mmap(control_output) but returned address = %p != 0x%08lx\n", pControl, CONTROL_OUTPUT_ADDR);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    shmem_control_output_address = (uint64_t *)pControl;
    precompile_read_address = &shmem_control_output_address[0];
    if (verbose) printf("mmap(control_output) mapped %lu B and returned address %p in %lu us\n", CONTROL_OUTPUT_SIZE, shmem_control_output_address, duration);
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
        printf("ERROR: Failed calling fopen(%s) errno=%d=%s; does it exist?\n", precompile_file_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Get input file size
    if (fseek(precompile_fp, 0, SEEK_END) == -1)
    {
        printf("ERROR: Failed calling fseek(%s) errno=%d=%s\n", precompile_file_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    long precompile_data_size = ftell(precompile_fp);
    if (precompile_data_size == -1)
    {
        printf("ERROR: Failed calling ftell(%s) errno=%d=%s\n", precompile_file_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if ((precompile_data_size & 0x7) != 0)
    {
        printf("ERROR: Precompile results file (%s) size (%lu) is not a multiple of 8 B\n", precompile_file_name, precompile_data_size);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }

    // Go back to the first byte
    if (fseek(precompile_fp, 0, SEEK_SET) == -1)
    {
        printf("ERROR: Failed calling fseek(%s, 0) errno=%d=%s\n", precompile_file_name, errno, strerror(errno));
        fflush(stdout);
        fflush(stderr);
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
                printf("ERROR: Size of precompile results file (%s) is too long (%lu)\n", precompile_file_name, precompile_data_size);
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            // Copy input data into input memory
            size_t precompile_read = fread(precompile_results_address, 1, precompile_data_size, precompile_fp);
            if (precompile_read != precompile_data_size)
            {
                printf("ERROR: Input read (%lu) != expected read size (%lu)\n", precompile_read, precompile_data_size);
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }

            // Initialize precompile written address
            *precompile_written_address = precompile_data_size >> 3; // in u64s

            //printf("Posting sem_prec_avail() precompile_written=%lu precompile_read=%lu\n", *precompile_written_address, *precompile_read_address);
            sem_post(sem_prec_avail);
        }
        else if (precompile_write_mode == PrecompileWriteMode_OnePrecAtATime)
        {
            // Check the precompile data size is inside the proper range
            if (precompile_data_size % (PRECOMPILE_FIXED_SIZE * 8) != 0)
            {
                printf("ERROR: Size of precompile results file (%s) is not a multiple %u * 8 B\n", precompile_file_name, PRECOMPILE_FIXED_SIZE);
                fflush(stdout);
                fflush(stderr);
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
                //printf("Waiting for sem_prec_read()\n");
                result = sem_wait(sem_prec_read);
                if (result == -1)
                {
                    printf("ERROR: Failed calling sem_wait(sem_prec_read) errno=%d=%s\n", errno, strerror(errno));
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
                }

                // Number of bytes to read from file and write to shared memory in every loop
                uint64_t bytes_to_read = sizeof(data);

                // Copy input data into input memory
                size_t precompile_read = fread(data, 1, bytes_to_read, precompile_fp);
                if (precompile_read != bytes_to_read)
                {
                    printf("ERROR: Input read (%lu) != expected read size (%lu)\n", precompile_read, bytes_to_read);
                    fflush(stdout);
                    fflush(stderr);
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

                //printf("Posting sem_prec_avail() precompile_written=%lu precompile_read=%lu\n", *precompile_written_address, *precompile_read_address);
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
            uint64_t bytes_to_read = sizeof(data);

            // Copy input data into input memory
            size_t precompile_read = fread(&data, 1, bytes_to_read, precompile_fp);
            if (precompile_read != bytes_to_read)
            {
                printf("ERROR: Input read (%lu) != expected read size (%lu)\n", precompile_read, bytes_to_read);
                fflush(stdout);
                fflush(stderr);
                exit(-1);
            }
            precompile_read_so_far += bytes_to_read;
            switch (data >> 32)
            {
                case CTRL_START:
                    //printf("Precompile CTRL_START\n");
                    assert(precompile_read_so_far == 8);
                    break;
                case CTRL_END:
                    //printf("Precompile CTRL_END\n");
                    assert(precompile_read_so_far == precompile_data_size);
                    break;
                // case CTRL_CANCEL:
                //     printf("Precompile CTRL_CANCEL\n");
                //     break;
                // case CTRL_ERROR:
                //     printf("Precompile CTRL_ERROR\n");
                //     break;
                case HINTS_TYPE_RESULT:
                {
                    //printf("Precompile HINTS_TYPE_RESULT\n");
                    if (precompile_write_mode == PrecompileWriteMode_OnePrecAtATime)
                    {
                        // Wait for server to read precompile results
                        //printf("Waiting for sem_prec_read()\n");
                        result = sem_wait(sem_prec_read);
                        if (result == -1)
                        {
                            printf("ERROR: Failed calling sem_wait(sem_prec_read) errno=%d=%s\n", errno, strerror(errno));
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                    }

                    uint64_t result_length = data & 0xFFFFFFFF;
                    if (result_length > (precompile_data_size - precompile_read_so_far))
                    {
                        printf("ERROR: Precompile HINTS_TYPE_RESULT length=%lu exceeds remaining file size %lu\n", result_length, precompile_data_size - precompile_read_so_far);
                        fflush(stdout);
                        fflush(stderr);
                        exit(-1);
                    }
                    //printf("Precompile HINTS_TYPE_RESULT result_length=%lu\n", result_length);
                    for (uint64_t i=0; i<result_length; i++)
                    {
                        uint64_t value;
                        size_t precompile_read = fread(&value, 1, 8, precompile_fp);
                        if (precompile_read != 8)
                        {
                            printf("ERROR: Input read (%lu) != expected read size (8)\n", precompile_read);
                            fflush(stdout);
                            fflush(stderr);
                            exit(-1);
                        }
                        memcpy(&precompile_results_address[(precompile_written_so_far >> 3) % (MAX_PRECOMPILE_SIZE >> 3)], &value, 8);
                        precompile_read_so_far += 8;
                        precompile_written_so_far += 8;
                        //printf("  Precompile result[%lu] = 0x%016lx\n", i, value);
                    }

                    if (precompile_write_mode == PrecompileWriteMode_OnePrecAtATime)
                    {
                        // Notify server that precompile results are available
                        *precompile_written_address = precompile_written_so_far >> 3; // in u64s

                        //printf("Posting sem_prec_avail() precompile_written=%lu precompile_read=%lu\n", *precompile_written_address, *precompile_read_address);
                        sem_post(sem_prec_avail);
                    }
                }
                break;
                // case HINTS_TYPE_ECRECOVER:
                //     {
                //         // Not implemented
                //         printf("Precompile HINTS_TYPE_ECRECOVER not implemented\n");
                //     }
                //     break;
                default:
                    printf("ERROR: Unknown precompile prefix type %lu\n", data >> 32);
                    fflush(stdout);
                    fflush(stderr);
                    exit(-1);
            }
        }

        if (precompile_write_mode == PrecompileWriteMode_Full)
        {
            // Notify server that precompile results are available
            *precompile_written_address = precompile_written_so_far >> 3; // in u64s

            //printf("Posting sem_prec_avail() precompile_written=%lu precompile_read=%lu\n", *precompile_written_address, *precompile_read_address);
            sem_post(sem_prec_avail);
        }

    }

    // Close the file pointer
    fclose(precompile_fp);

#ifdef DEBUG
    gettimeofday(&stop_time, NULL);
    duration = TimeDiff(start_time, stop_time);
    printf("client (precompile): done in %lu us\n", duration);
#endif
}

void client_run (void)
{
    printf("client_run(): Starting client...\n");
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
            printf("ERROR: Failed calling input shm_open(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
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

        // Write the free input value as 0 in the first 64 bits
        *(uint64_t *)shmem_input_address = (uint64_t)0; // free input

        // Copy input data into input memory
        size_t input_read = fread(shmem_input_address + 8, 1, input_data_size, input_fp);
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

        // Set written counter
        *input_written_address = input_data_size; // in bytes

#ifdef DEBUG
        gettimeofday(&stop_time, NULL);
        duration = TimeDiff(start_time, stop_time);
        printf("client (input): done in %lu us\n", duration);
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
    if (response[0] != TYPE_PONG)
    {
        printf("ERROR: recv() returned unexpected type=%lu\n", response[0]);
        fflush(stdout);
        fflush(stderr);
        exit(-1);
    }
    if (response[1] != gen_method)
    {
        printf("ERROR: recv() returned unexpected gen_method=%lu\n", response[1]);
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
                request[1] = MAX_STEPS;
                request[2] = 1ULL << 18; // chunk_len
                request[3] = 0;
                request[4] = 0;

                if (precompile_results_enabled)
                {
                    client_write_precompile_results();
                }

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
                request[1] = MAX_STEPS;
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

                if (precompile_results_enabled)
                {
                    client_write_precompile_results();
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
                request[1] = MAX_STEPS;
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

                if (precompile_results_enabled)
                {
                    client_write_precompile_results();
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
                request[1] = MAX_STEPS;
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
                    request[1] = MAX_STEPS;
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
                        request[1] = MAX_STEPS;
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
                request[1] = MAX_STEPS;
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
                request[1] = MAX_STEPS;
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
                    request[1] = MAX_STEPS;
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
                        request[1] = MAX_STEPS;
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