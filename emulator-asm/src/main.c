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

// Assembly-provided functions
void emulator_start(void);
uint64_t get_max_bios_pc(void);
uint64_t get_max_program_pc(void);
uint64_t get_gen_method(void);

#define RAM_ADDR (uint64_t)0xa0000000
#define RAM_SIZE (uint64_t)0x08000000 // 128MB
#define SYS_ADDR RAM_ADDR
#define SYS_SIZE (uint64_t)0x10000
#define OUTPUT_ADDR (SYS_ADDR + SYS_SIZE)

#define ROM_ADDR (uint64_t)0x80000000
#define ROM_SIZE (uint64_t)0x08000000 // 128MB

#define INPUT_ADDR (uint64_t)0x90000000
#define MAX_INPUT_SIZE (uint64_t)0x08000000 // 128MB

#define TRACE_ADDR         (uint64_t)0xb0000000
#define INITIAL_TRACE_SIZE (uint64_t)0x100000000 // 4GB

#define REG_ADDR (uint64_t)0x70000000
#define REG_SIZE (uint64_t)0x1000 // 4kB

struct timeval start_time;

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

void log_minimal_trace(void);
void log_histogram(void);
void log_main_trace(void);

// Configuration
bool output = true;
bool metrics = false;
bool trace = false;
bool trace_trace = false;
#ifdef DEBUG
bool verbose = false;
#endif
char * input_parameter = NULL;
bool is_file = false;
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

// Maximum length of the shared memory prefix, e.g. SHMZISK12345678
#define MAX_SHM_PREFIX_LENGTH 32

// Input shared memory
char * shmem_input_sufix = "_input";
char shmem_input_name[128];
int shmem_input_fd = -1;
uint64_t shmem_input_size = 0;
void * shmem_input_address = NULL;

// Input semaphore: notifies the caller when the trace is ready to be consumed
char * sem_input_sufix = "_semin";
char sem_input_name[128];
sem_t * sem_input = NULL;

// Output shared memory
char * shmem_output_sufix = "_output";
char shmem_output_name[128];
int shmem_output_fd = -1;

// Output semaphore: lets the caller notify that the trace has been consumed and it can be unlinked
char * sem_output_sufix = "_semout";
char sem_output_name[128];
sem_t * sem_output = NULL;

// Chunk done semaphore: notifies the caller when a new chunk has been processed
char * sem_chunk_done_sufix = "_semckd";
char sem_chunk_done_name[128];
sem_t * sem_chunk_done = NULL;

int process_id = 0;

uint64_t input_size = 0;

int main(int argc, char *argv[])
{
    // Result, to be used in calls to functions returning int
    int result;

    // Get current process id
    process_id = getpid();

    // Parse arguments
    parse_arguments(argc, argv);

    // Check if the input parameter is a shared memory ID or a file name
    if (strncmp(input_parameter, "ZISK", 4) == 0)
    {
        // Mark this is a shared memory, i.e. it is not a file
        is_file = false;

        // Check the length of the input parameter, which is a prefix to be used to build
        // shared memory region names
        uint64_t input_parameter_length = strlen(input_parameter);
        if (input_parameter_length > MAX_SHM_PREFIX_LENGTH)
        {
            printf("Input parameter is too long: %s, size = %lu\n", input_parameter, input_parameter_length);
            return -1;
        }

        // Build shared memory region names
        char shmem_prefix[128];
        strcpy(shmem_prefix, "/");
        strcat(shmem_prefix, input_parameter);
        strcpy(shmem_input_name, shmem_prefix);
        strcat(shmem_input_name, shmem_input_sufix);
        strcpy(shmem_output_name, shmem_prefix);
        strcat(shmem_output_name, shmem_output_sufix);

        // Build the semaphore names
        strcpy(sem_input_name, shmem_prefix);
        strcat(sem_input_name, sem_input_sufix);
        strcpy(sem_output_name, shmem_prefix);
        strcat(sem_output_name, sem_output_sufix);
        if (generate_minimal_trace || generate_main_trace || generate_zip)
        {
            strcpy(sem_chunk_done_name, shmem_prefix);
            strcat(sem_chunk_done_name, sem_chunk_done_sufix);
        }

        // Create (or open if existing) input and output semaphores
        sem_input = sem_open(sem_input_name, O_CREAT, 0644, 1);
        if (sem_input == SEM_FAILED)
        {
            printf("Failed calling sem_open(%s) errno=%d=%s\n", sem_input_name, errno, strerror(errno));
            return -1;
        }
        sem_output = sem_open(sem_output_name, O_CREAT, 0644, 1);
        if (sem_input == SEM_FAILED)
        {
            printf("Failed calling sem_open(%s) errno=%d=%s\n", sem_output_name, errno, strerror(errno));
            return -1;
        }
        if (generate_minimal_trace || generate_main_trace || generate_zip)
        {
            sem_chunk_done = sem_open(sem_chunk_done_name, O_CREAT, 0644, 1);
            if (sem_chunk_done == SEM_FAILED)
            {
                printf("Failed calling sem_open(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
                return -1;
            }
        }

#ifdef DEBUG
        if (verbose) printf("Emulator C start; input shared memory ID = %s\n", input_parameter);
#endif        
    }
    else
    {
        // Mark this is an input file
        is_file = true;
        sprintf(shmem_output_name, "ZISK_%d_output", process_id);
#ifdef DEBUG
        if (verbose) printf("Emulator C start; input file = %s shmem=%s\n", input_parameter, shmem_output_name);
#endif
    }

    /*********/
    /* INPUT */
    /*********/
    // Allocate input memory region and initialize it with the data coming from the file
    // of from the input shared memory region
    if (is_file)
    {
        // Open input file
        FILE * input_fp = fopen(input_parameter, "r");
        if (input_fp == NULL)
        {
            printf("Failed calling fopen(%s) errno=%d=%s; does it exist?\n", input_parameter, errno, strerror(errno));
            return -1;
        }

        // Get input file size
        if (fseek(input_fp, 0, SEEK_END) == -1)
        {
            printf("Failed calling fseek(%s) errno=%d=%s\n", input_parameter, errno, strerror(errno));
            return -1;
        }
        long input_data_size = ftell(input_fp);
        if (input_data_size == -1)
        {
            printf("Failed calling ftell(%s) errno=%d=%s\n", input_parameter, errno, strerror(errno));
            return -1;
        }

        // Go back to the first byte
        if (fseek(input_fp, 0, SEEK_SET) == -1)
        {
            printf("Failed calling fseek(%s, 0) errno=%d=%s\n", input_parameter, errno, strerror(errno));
            return -1;
        }

        // Check the input data size is inside the proper range
        if (input_data_size > (MAX_INPUT_SIZE - 8))
        {
            printf("Size of input file (%s) is too long (%lu)\n", input_parameter, input_data_size);
            return -1;
        }

        // Calculate input size = input file data size + 8B for size header + round up to higher 8B
        // boundary
        input_size = ((input_data_size + 16 + 7) >> 3) << 3;

        // Map input address space
        void * pInput = mmap((void *)INPUT_ADDR, input_size, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
        if (pInput == NULL)
        {
            printf("Failed calling mmap(input) errno=%d=%s\n", errno, strerror(errno));
            return -1;
        }
        if ((uint64_t)pInput != INPUT_ADDR)
        {
            printf("Called mmap(pInput) but returned address = 0x%p != 0x%lx\n", pInput, INPUT_ADDR);
            return -1;
        }
    #ifdef DEBUG
        if (verbose) printf("mmap(input) returned 0x%p\n", pInput);
    #endif

        // Write the input size in the first 64 bits
        *(uint64_t *)INPUT_ADDR = (uint64_t)0; // free input
        *(uint64_t *)(INPUT_ADDR + 8) = (uint64_t)input_data_size;

        // Copy input data into input memory
        size_t input_read = fread((void *)(INPUT_ADDR + 16), 1, input_data_size, input_fp);
        if (input_read != input_data_size)
        {
            printf("Input read (%lu) != input file size (%lu)\n", input_read, input_data_size);
            return -1;
        }

        // Close the file pointer
        fclose(input_fp);
    }
    else
    {
        // Open input shared memory
        shmem_input_fd = shm_open(shmem_input_name, /*O_RDWR*/ O_RDONLY, 0666);
        if (shmem_input_fd < 0)
        {
            printf("Failed calling shm_open(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            return -1;
        }

        // Map the shared memory object into the process address space, but just the 32B header
        shmem_input_address = mmap(NULL, 32, PROT_READ, MAP_SHARED, shmem_input_fd, 0);
        if (shmem_input_address == MAP_FAILED)
        {
            printf("Failed calling mmap(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            return -1;
        }

        // Read input header data
        uint64_t * control = (uint64_t *)shmem_input_address;
        if (generate_minimal_trace || generate_zip) {
            chunk_size = control[0];
            assert(chunk_size > 0);
            chunk_size_mask = chunk_size - 1;
        }
        max_steps = control[1];
        assert(max_steps > 0);
        initial_trace_size = control[2]; // Initial trace size
        assert(initial_trace_size > 0);
        trace_size = initial_trace_size;
        trace_address_threshold = TRACE_ADDR + initial_trace_size - MAX_CHUNK_TRACE_SIZE;
        shmem_input_size = control[3];

        // Unmap input header
        result = munmap(shmem_input_address, 32);
        if (result == -1)
        {
            printf("Failed calling munmap(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            exit(-1);
        }
        
        // Map the shared memory object into the process address space
        shmem_input_address = mmap(NULL, shmem_input_size + 32, PROT_READ /*| PROT_WRITE*/, MAP_SHARED, shmem_input_fd, 0);
        if (shmem_input_address == MAP_FAILED)
        {
            printf("Failed calling mmap(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            return -1;
        }

        // Calculate input size
        input_size = ((shmem_input_size + 16 + 7) >> 3) << 3;

        // Map input address space
        void * pInput = mmap((void *)INPUT_ADDR, input_size, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
        if (pInput == NULL)
        {
            printf("Failed calling mmap(input) errno=%d=%s\n", errno, strerror(errno));
            return -1;
        }
        if ((uint64_t)pInput != INPUT_ADDR)
        {
            printf("Called mmap(pInput) but returned address = 0x%p != 0x%lx\n", pInput, INPUT_ADDR);
            return -1;
        }
#ifdef DEBUG
        if (verbose) printf("mmap(input) returned 0x%p\n", pInput);
#endif

        // Write the input size in the first 64 bits
        *(uint64_t *)INPUT_ADDR = (uint64_t)0; // free input
        *(uint64_t *)(INPUT_ADDR + 8)= (uint64_t)shmem_input_size;

        // Copy the input data
        memcpy((void *)(INPUT_ADDR + 16), shmem_input_address + 32, shmem_input_size);

        // Unmap input
        result = munmap(shmem_input_address, shmem_input_size + 32);
        if (result == -1)
        {
            printf("Failed calling munmap(%s) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            exit(-1);
        }
        
        // Unlink input
        result = shm_unlink(shmem_input_name);
        if (result == -1)
        {
            printf("Failed calling shm_unlink(%s) size=%lu errno=%d=%s\n", shmem_input_name, trace_size, errno, strerror(errno));
            exit(-1);
        }
    }

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
            return -1;
        }

        // Size it
        result = ftruncate(shmem_output_fd, trace_size);
        if (result != 0)
        {
            printf("Failed calling ftruncate(%s) errno=%d=%s\n", shmem_output_name, errno, strerror(errno));
            return -1;
        }

        // Map it to the trace address
        void * pTrace = mmap((void *)TRACE_ADDR, trace_size, PROT_READ | PROT_WRITE, MAP_SHARED, shmem_output_fd, 0);
        if (pTrace == NULL)
        {
            printf("Failed calling mmap(pTrace) errno=%d=%s\n", errno, strerror(errno));
            return -1;
        }
        if ((uint64_t)pTrace != TRACE_ADDR)
        {
            printf("Called mmap(trace) but returned address = 0x%p != 0x%lx\n", pTrace, TRACE_ADDR);
            return -1;
        }
    #ifdef DEBUG
        if (verbose) printf("mmap(trace) returned 0x%p\n", pTrace);
    #endif

        // Init output header data
        uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
        pOutput[0] = 0x000100; // Version, e.g. v1.0.0 [8]
        pOutput[1] = 1; // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
        // MT allocated size [8] -> to be updated after completion
        // MT used size [8] -> to be updated after completion
    }
    
    /*******/
    /* RAM */
    /*******/
    void * pRam = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
    if (pRam == NULL)
    {
        printf("Failed calling mmap(ram) errno=%d=%s\n", errno, strerror(errno));
        return -1;
    }
    if ((uint64_t)pRam != RAM_ADDR)
    {
        printf("Called mmap(ram) but returned address = 0x%p != 0x%08lx\n", pRam, RAM_ADDR);
        return -1;
    }
#ifdef DEBUG
    if (verbose) printf("mmap(ram) returned 0x%p\n", pRam);
#endif

    /*******/
    /* ROM */
    /*******/
    void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
    if (pRom == NULL)
    {
        printf("Failed calling mmap(rom) errno=%d=%s\n", errno, strerror(errno));
        return -1;
    }
    if ((uint64_t)pRom != ROM_ADDR)
    {
        printf("Called mmap(rom) but returned address = 0x%p != 0x%lx\n", pRom, ROM_ADDR);
        return -1;
    }
#ifdef DEBUG
    if (verbose) printf("mmap(rom) returned 0x%p\n", pRom);
#endif

    /*******/
    /* ASM */
    /*******/
    // Call emulator assembly code
    gettimeofday(&start_time,NULL);
    emulator_start();
    struct timeval stop_time;
    gettimeofday(&stop_time,NULL);

    uint64_t final_trace_size = MEM_CHUNK_ADDRESS - MEM_TRACE_ADDRESS;

    if ( metrics
#ifdef DEBUG
        || keccak_metrics
#endif
        )
    {
        uint64_t duration = TimeDiff(start_time, stop_time);
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
    if (!is_file)
    {
        result = sem_post(sem_input);
        if (result == -1)
        {
            printf("Failed calling sem_post(%s) errno=%d=%s\n", sem_input_name, errno, strerror(errno));
            exit(-1);
        }
    }

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

#ifdef DEBUG
    if (verbose) printf("Emulator C end\n");
#endif

    /************/
    /* CLEAN UP */
    /************/

    // Cleanup ROM
    result = munmap((void *)ROM_ADDR, ROM_SIZE);
    if (result == -1)
    {
        printf("Failed calling munmap(rom) errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }

    // Cleanup RAM
    result = munmap((void *)RAM_ADDR, RAM_SIZE);
    if (result == -1)
    {
        printf("Failed calling munmap(ram) errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }

    // Cleanup INPUT
    result = munmap((void *)INPUT_ADDR, input_size);
    if (result == -1)
    {
        printf("Failed calling munmap(input) errno=%d=%s\n", errno, strerror(errno));
        exit(-1);
    }

    // Cleanup trace
    if (generate_minimal_trace || generate_rom_histogram || generate_zip)
    {
        result = munmap((void *)TRACE_ADDR, trace_size);
        if (result == -1)
        {
            printf("Failed calling munmap(trace) for size=%lu errno=%d=%s\n", trace_size, errno, strerror(errno));
            exit(-1);
        }

        // Wait for caller to notify when the trace has been totally consumed
        if (!is_file)
        {
            //printf("C sem_wait(%s)...\n", sem_output_name);
            result = sem_wait(sem_output);
            //printf("C sem_wait(%s) done\n", sem_output_name);
            if (result == -1)
            {
                printf("Failed calling sem_wait(%s) errno=%d=%s\n", sem_output_name, errno, strerror(errno));
                exit(-1);
            }
        }
    }

    // Make sure the output shared memory is deleted
    shm_unlink(shmem_output_name);

    // Cleanup semaphores
    if (!is_file)
    {
        result = sem_close(sem_input);
        if (result == -1)
        {
            printf("Failed calling sem_close(%s) errno=%d=%s\n", sem_input_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_input_name);
        if (result == -1)
        {
            printf("Failed calling sem_unlink(%s) errno=%d=%s\n", sem_input_name, errno, strerror(errno));
        }
        result = sem_close(sem_output);
        if (result == -1)
        {
            printf("Failed calling sem_close(%s) errno=%d=%s\n", sem_output_name, errno, strerror(errno));
        }
        result = sem_unlink(sem_output_name);
        if (result == -1)
        {
            printf("Failed calling sem_unlink(%s) errno=%d=%s\n", sem_output_name, errno, strerror(errno));
        }
        if (generate_minimal_trace || generate_main_trace || generate_zip)
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
}

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
extern uint64_t reg_32;
extern uint64_t reg_33;
extern uint64_t reg_34;

extern int _print_regs()
{
    printf("print_regs()\n");
    printf("\treg[ 0]=%lu=0x%lx=@%p\n", reg_0,  reg_0,  &reg_0);
    //printf("\treg[ 1]=%lu=0x%lx=@%p\n", reg_1,  reg_1,  &reg_1);
    //printf("\treg[ 2]=%lu=0x%lx=@%p\n", reg_2,  reg_2,  &reg_2);
    printf("\treg[ 3]=%lu=0x%lx=@%p\n", reg_3,  reg_3,  &reg_3);
    printf("\treg[ 4]=%lu=0x%lx=@%p\n", reg_4,  reg_4,  &reg_4);
    /*printf("\treg[ 5]=%lu=0x%lx=@%p\n", reg_5,  reg_5,  &reg_5);
    printf("\treg[ 6]=%lu=0x%lx=@%p\n", reg_6,  reg_6,  &reg_6);
    printf("\treg[ 7]=%lu=0x%lx=@%p\n", reg_7,  reg_7,  &reg_7);
    printf("\treg[ 8]=%lu=0x%lx=@%p\n", reg_8,  reg_8,  &reg_8);
    printf("\treg[ 9]=%lu=0x%lx=@%p\n", reg_9,  reg_9,  &reg_9);
    printf("\treg[10]=%lu=0x%lx=@%p\n", reg_10, reg_10, &reg_10);
    printf("\treg[11]=%lu=0x%lx=@%p\n", reg_11, reg_11, &reg_11);
    printf("\treg[12]=%lu=0x%lx=@%p\n", reg_12, reg_12, &reg_12);
    printf("\treg[13]=%lu=0x%lx=@%p\n", reg_13, reg_13, &reg_13);
    printf("\treg[14]=%lu=0x%lx=@%p\n", reg_14, reg_14, &reg_14);
    printf("\treg[15]=%lu=0x%lx=@%p\n", reg_15, reg_15, &reg_15);
    printf("\treg[16]=%lu=0x%lx=@%p\n", reg_16, reg_16, &reg_16);
    printf("\treg[17]=%lu=0x%lx=@%p\n", reg_17, reg_17, &reg_17);
    printf("\treg[18]=%lu=0x%lx=@%p\n", reg_18, reg_18, &reg_18);*/
    printf("\treg[19]=%lu=0x%lx=@%p\n", reg_19, reg_19, &reg_19);
    printf("\treg[20]=%lu=0x%lx=@%p\n", reg_20, reg_20, &reg_20);
    printf("\treg[21]=%lu=0x%lx=@%p\n", reg_21, reg_21, &reg_21);
    printf("\treg[22]=%lu=0x%lx=@%p\n", reg_22, reg_22, &reg_22);
    printf("\treg[23]=%lu=0x%lx=@%p\n", reg_23, reg_23, &reg_23);
    printf("\treg[24]=%lu=0x%lx=@%p\n", reg_24, reg_24, &reg_24);
    printf("\treg[25]=%lu=0x%lx=@%p\n", reg_25, reg_25, &reg_25);
    printf("\treg[26]=%lu=0x%lx=@%p\n", reg_26, reg_26, &reg_26);
    printf("\treg[27]=%lu=0x%lx=@%p\n", reg_27, reg_27, &reg_27);
    printf("\treg[28]=%lu=0x%lx=@%p\n", reg_28, reg_28, &reg_28);
    printf("\treg[29]=%lu=0x%lx=@%p\n", reg_29, reg_29, &reg_29);
    printf("\treg[30]=%lu=0x%lx=@%p\n", reg_30, reg_30, &reg_30);
    printf("\treg[31]=%lu=0x%lx=@%p\n", reg_31, reg_31, &reg_31);
    printf("\treg[32]=%lu=0x%lx=@%p\n", reg_32, reg_32, &reg_32);
    printf("\treg[33]=%lu=0x%lx=@%p\n", reg_33, reg_33, &reg_33);
    printf("\treg[34]=%lu=0x%lx=@%p\n", reg_34, reg_34, &reg_34);
    printf("\n");
}

extern void _chunk_done()
{
    // Notify the caller that a new chunk is done and its trace is ready to be consumed
    if (!is_file)
    {
        assert(generate_minimal_trace || generate_main_trace || generate_zip);
        int result = sem_post(sem_chunk_done);
        if (result == -1)
        {
            printf("Failed calling sem_post(%s) errno=%d=%s\n", sem_chunk_done_name, errno, strerror(errno));
            exit(-1);
        }
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
        exit(-1);
    }
    
    // Remap the memory
    void * new_address = mremap((void *)trace_address, trace_size, new_trace_size, 0);
    if ((uint64_t)new_address != trace_address)
    {
        printf("realloc_trace() failed calling mremap() from size=%lu to %lu got new_address=0x%p errno=%d=%s\n", trace_size, new_trace_size, new_address, errno, strerror(errno));
        exit(-1);
    }

    // Update trace global variables
    trace_size = new_trace_size;
    trace_address_threshold = TRACE_ADDR + trace_size - MAX_CHUNK_TRACE_SIZE;
}

void print_usage (void)
{
#ifdef DEBUG
    printf("Usage: ziskemuasm <input_file> [--gen=0|--generate_fast] [--gen=1|--generate_minimal_trace] [--gen=2|--generate_rom_histogram] [--gen=3|--generate_main_trace] [--gen=4|--generate_chunks] [--gen=6|--generate_zip] [-o output off] [-m metrics on] [-t trace on] [-tt trace on] [-v verbose on] [-k keccak trace on] [-h/--help print this]\n");
#else
    printf("Usage: ziskemuasm <input_file> [--gen=0|--generate_fast] [--gen=1|--generate_minimal_trace] [--gen=2|--generate_rom_histogram] [--gen=3|--generate_main_trace] [--gen=4|--generate_chunks] [--gen=6|--generate_zip] [-o output off] [-m metrics on] [-t trace on] [-tt trace on] [-h/--help print this]\n");
#endif
}

uint64_t get_c_gen_method(void)
{
    if (generate_fast) return 0;
    if (generate_minimal_trace) return 1;
    if (generate_rom_histogram) return 2;
    if (generate_main_trace) return 3;
    if (generate_chunks) return 4;
    if (generate_zip) return 6;
    printf("get_c_gen_method() called without any generation method active\n");
    exit(-1);
}
void parse_arguments(int argc, char *argv[])
{
    uint64_t number_of_selected_generation_methods = 0;
    if (argc > 1)
    {
        for (int i = 1; i < argc; i++)
        {
            if ( (strcmp(argv[i], "--gen=0") == 0) || (strcmp(argv[i], "--generate_fast") == 0))
            {
                generate_fast = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=1") == 0) || (strcmp(argv[i], "--generate_minimal_trace") == 0))
            {
                generate_minimal_trace = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=2") == 0) || (strcmp(argv[i], "--generate_rom_histogram") == 0))
            {
                generate_rom_histogram = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=3") == 0) || (strcmp(argv[i], "--generate_main_trace") == 0))
            {
                generate_main_trace = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=4") == 0) || (strcmp(argv[i], "--generate_chunks") == 0))
            {
                generate_chunks = true;
                number_of_selected_generation_methods++;
                continue;
            }
            if ( (strcmp(argv[i], "--gen=6") == 0) || (strcmp(argv[i], "--generate_zip") == 0))
            {
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
            // We accept only one input parameter (beyond the flags)
            if (input_parameter == NULL)
            {
                input_parameter = argv[i];
                continue;
            }
            printf("Unrecognized argument: %s, current input=%s\n", argv[i], input_parameter);
            print_usage();
            exit(-1);
        }
    }
    
    if (number_of_selected_generation_methods != 1)
    {
        printf("Invalid arguments: select 1 generation method, and only one\n");
        print_usage();
        exit(-1);
    }

    uint64_t asm_gen_method = get_gen_method();
    uint64_t c_gen_method = get_c_gen_method();
    if (asm_gen_method != c_gen_method)
    {
        printf("Inconsistency: C generation method is %lu but ASM generation method is %lu\n",
            c_gen_method,
            asm_gen_method);
        print_usage();
        exit(-1);
    }
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