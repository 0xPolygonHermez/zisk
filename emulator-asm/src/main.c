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

void emulator_start(void);

#define RAM_ADDR 0xa0000000
#define RAM_SIZE 0x08000000 // 128MB
#define SYS_ADDR RAM_ADDR
#define SYS_SIZE 0x10000
#define OUTPUT_ADDR (SYS_ADDR + SYS_SIZE)

#define ROM_ADDR 0x80000000
#define ROM_SIZE 0x08000000 // 128MB

#define INPUT_ADDR 0x90000000
#define MAX_INPUT_SIZE 0x08000000 // 128MB

#define TRACE_ADDR         (uint64_t)0xb0000000
#define INITIAL_TRACE_SIZE (uint64_t)0x100000000 // 4GB

#define REG_ADDR 0x70000000
#define REG_SIZE 0x1000 // 4kB

struct timeval start_time;

extern uint64_t MEM_STEP;
extern uint64_t MEM_END;
extern uint64_t MEM_TRACE_ADDRESS;
extern uint64_t MEM_CHUNK_ADDRESS;
extern uint64_t MEM_CHUNK_START_STEP;

struct timeval keccak_start, keccak_stop;
uint64_t keccak_counter = 0;
uint64_t keccak_duration = 0;

uint64_t realloc_counter = 0;

uint64_t printed_chars_counter = 0;

extern void keccakf1600_generic(uint64_t state[25]);

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
#ifdef DEBUG
void log_trace(void);
#endif

// Configuration
bool output = true;
bool metrics = false;
#ifdef DEBUG
bool verbose = false;
bool trace = false;
bool trace_trace = false;
bool keccak_metrics = false;
#endif
char * input_parameter = NULL;
bool is_file = false;
bool generate_traces = true;

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
    if (strncmp(input_parameter, "SHM", 3) == 0)
    {
        // Mark this is a shared memory, i.e. it is not a file
        is_file = false;

        // Check the length of the input parameter, which is a prefix to be used to build
        // shared memory region names
        uint64_t input_parameter_length = strlen(input_parameter);
        if (input_parameter_length > 16)
        {
            printf("Input parameter is too long: %s, size = %d\n", input_parameter, input_parameter_length);
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
#ifdef DEBUG
        if (verbose) printf("Emulator C start; input shared memory ID = %s\n", input_parameter);
#endif        
    }
    else
    {
        // Mark this is an input file
        is_file = true;
        sprintf(shmem_output_name, "SHM_%d_output", process_id);
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
            printf("Size of input file (%s) is too long (%d)\n", input_parameter, input_data_size);
            return -1;
        }

        // Calculate input size = input file data size + 8B for size header + round up to higher 8B
        // boundary
        input_size = ((input_data_size + 8 + 7) >> 3) << 3;

        // Map input address space
        void * pInput = mmap((void *)INPUT_ADDR, input_size, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
        if (pInput == NULL)
        {
            printf("Failed calling mmap(input) errno=%d=%s\n", errno, strerror(errno));
            return -1;
        }
        if ((uint64_t)pInput != INPUT_ADDR)
        {
            printf("Called mmap(pInput) but returned address = 0x%llx != 0x%llx\n", pInput, INPUT_ADDR);
            return -1;
        }
    #ifdef DEBUG
        if (verbose) printf("mmap(input) returned %08x\n", pInput);
    #endif

        // Write the input size in the first 64 bits
        *(uint64_t *)INPUT_ADDR = (uint64_t)input_data_size;

        // Copy input data into input memory
        size_t input_read = fread((void *)(INPUT_ADDR + 8), 1, input_data_size, input_fp);
        if (input_read != input_data_size)
        {
            printf("Input read (%d) != input file size (%d)\n", input_read, input_data_size);
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
        chunk_size = control[0];
        assert(chunk_size > 0);
        chunk_size_mask = chunk_size - 1;
        max_steps = control[1];
        assert(max_steps > 0);
        initial_trace_size = control[2]; // Initial trace size
        assert(initial_trace_size > 0);
        trace_size = initial_trace_size;
        shmem_input_size = control[3];

        // Unmap input header
        result = munmap(shmem_input_address, 32);
        if (result == -1)
        {
            printf("Failed calling munmap({}) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
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
        input_size = ((shmem_input_size + 8 + 7) >> 3) << 3;

        // Map input address space
        void * pInput = mmap((void *)INPUT_ADDR, input_size, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
        if (pInput == NULL)
        {
            printf("Failed calling mmap(input) errno=%d=%s\n", errno, strerror(errno));
            return -1;
        }
        if ((uint64_t)pInput != INPUT_ADDR)
        {
            printf("Called mmap(pInput) but returned address = 0x%llx != 0x%llx\n", pInput, INPUT_ADDR);
            return -1;
        }
#ifdef DEBUG
        if (verbose) printf("mmap(input) returned %08x\n", pInput);
#endif

        // Write the input size in the first 64 bits
        *(uint64_t *)INPUT_ADDR = (uint64_t)shmem_input_size;

        // Copy the input data
        memcpy((void *)(INPUT_ADDR + 8), shmem_input_address + 32, shmem_input_size);

        // Unmap input
        result = munmap(shmem_input_address, shmem_input_size + 32);
        if (result == -1)
        {
            printf("Failed calling munmap({}) errno=%d=%s\n", shmem_input_name, errno, strerror(errno));
            exit(-1);
        }
        
        // Unlink input
        result = shm_unlink(shmem_input_name);
        if (result == -1)
        {
            printf("Failed calling shm_unlink(%s) errno=%d=%s\n", shmem_input_name, trace_size, errno, strerror(errno));
            exit(-1);
        }
    }

    /*********/
    /* TRACE */
    /*********/

    if (generate_traces)
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
            printf("Failed calling ftruncate() errno=%d=%s\n", shmem_output_name, errno, strerror(errno));
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
            printf("Called mmap(trace) but returned address = 0x%llx != 0x%llx\n", pTrace, TRACE_ADDR);
            return -1;
        }
    #ifdef DEBUG
        if (verbose) printf("mmap(trace) returned %08x\n", pTrace);
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
        printf("Called mmap(ram) but returned address = 0x%08x != 0x%08x\n", pRam, RAM_ADDR);
        return -1;
    }
#ifdef DEBUG
    if (verbose) printf("mmap(ram) returned %08x\n", pRam);
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
        printf("Called mmap(rom) but returned address = 0x%llx != 0x%llx\n", pRom, ROM_ADDR);
        return -1;
    }
#ifdef DEBUG
    if (verbose) printf("mmap(rom) returned %08x\n", pRom);
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
        printf("Duration = %lld us, Keccak counter = %lld, realloc counter = %lld, steps = %lld, step duration = %lld ns, tp = %lld steps/s, trace size = 0x%llx - 0x%llx = %lld B(%d%%), end=%lld\n",
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
            printf("Keccak counter = %lld, duration = %lld us, single keccak duration = %lld ns, percentage = %lld \n", keccak_counter, keccak_duration, single_keccak_duration_ns, keccak_percentage);
        }
#else
        printf("Duration = %lld us, realloc counter = %lld, steps = %lld, step duration = %lld ns, tp = %lld steps/s, trace size = 0x%llx - 0x%llx = %lld B(%d%%), end=%lld\n",
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
    if (generate_traces)
    {
        uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
        pOutput[0] = 0x000100; // Version, e.g. v1.0.0 [8]
        pOutput[1] = 0; // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
        pOutput[2] = trace_size; // MT allocated size [8]
        //assert(final_trace_size > 32);
        pOutput[3] = final_trace_size - 32; // MT used size [8]
    }

    // Log trace
#ifdef DEBUG
    if (generate_traces && trace)
    {
        log_trace();
    }

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
    if (generate_traces)
    {
        result = munmap((void *)TRACE_ADDR, trace_size);
        if (result == -1)
        {
            printf("Failed calling munmap(trace) for size=%d errno=%d=%s\n", trace_size, errno, strerror(errno));
            exit(-1);
        }

        if (is_file)
        {
            // Make sure the output shared memory is deleted
            shm_unlink(shmem_output_name);
        }
    }
}

uint64_t print_abcflag_counter = 0;

extern int _print_abcflag(uint64_t a, uint64_t b, uint64_t c, uint64_t flag)
{
    uint64_t * pMem = (uint64_t *)0xa0012118;
    printf("counter=%d a=%08llx b=%08llx c=%08llx flag=%08llx mem=%08llx\n", print_abcflag_counter, a, b, c, flag, *pMem);
    uint64_t *pRegs = (uint64_t *)RAM_ADDR;
    for (int i=0; i<32; i++)
    {
        printf("r%d=%08llx ", i, pRegs[i]);
    }
    printf("\n");
    fflush(stdout);
    print_abcflag_counter++;
    return 0;
}

extern int _print_char(uint64_t param)
{
    printed_chars_counter++;
    char c = param;
    printf("%c", c);
    return 0;
}

uint64_t print_step_counter = 0;
extern int _print_step(uint64_t step)
{
#ifdef DEBUG
    printf("step=%d\n", print_step_counter);
    print_step_counter++;
    // struct timeval stop_time;
    // gettimeofday(&stop_time,NULL);
    // uint64_t duration = TimeDiff(start_time, stop_time);
    // uint64_t duration_s = duration/1000;
    // if (duration_s == 0) duration_s = 1;
    // uint64_t speed = step / duration_s;
    // if (verbose) printf("print_step() Counter=%d Step=%d Duration=%dus Speed=%dsteps/ms\n", print_step_counter, step, duration, speed);
#endif
    return 0;
}

extern int _opcode_keccak(uint64_t address)
{
#ifdef DEBUG
    if (keccak_metrics || verbose) gettimeofday(&keccak_start, NULL);
#endif
    //if (verbose) printf("opcode_keccak() calling KeccakF1600() counter=%d step=%08llx address=%08llx\n", keccak_counter, /**(uint64_t *)*/MEM_STEP, address);
    keccakf1600_generic((uint64_t *)address);
    //zisk_keccakf((uint64_t *)address);
    //if (verbose) printf("opcode_keccak() called KeccakF1600()\n");
#ifdef DEBUG
    keccak_counter++;
    if (keccak_metrics || verbose)
    {
        gettimeofday(&keccak_stop, NULL);
        keccak_duration += TimeDiff(keccak_start, keccak_stop);
    }
#endif
    return 0;
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
        printf("realloc_trace() failed calling ftruncate(%s) of new size=%lld errno=%d=%s\n", shmem_output_name, new_trace_size, errno, strerror(errno));
        exit(-1);
    }
    
    // Remap the memory
    void * new_address = mremap((void *)trace_address, trace_size, new_trace_size, 0);
    if ((uint64_t)new_address != trace_address)
    {
        printf("realloc_trace() failed calling mremap() from size=%d to %d got new_address=0x%llx errno=%d=%s\n", trace_size, new_trace_size, new_address, errno, strerror(errno));
        exit(-1);
    }

    // Update trace global variables
    trace_size = new_trace_size;
    trace_address_threshold = TRACE_ADDR + trace_size - MAX_CHUNK_TRACE_SIZE;
}

void print_usage (void)
{
#ifdef DEBUG
    printf("Usage: ziskemuasm <input_file> [-o output off] [-m metrics on] [-v verbose on] [-t trace on] [-tt trace on] [-k keccak trace on] [-h/--help print this]\n");
#else
    printf("Usage: ziskemuasm <input_file> [-o output off] [-m metrics on] [-h/--help print this]\n");
#endif
}

void parse_arguments(int argc, char *argv[])
{
    if (argc > 1)
    {
        for (int i = 1; i < argc; i++)
        {
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
#ifdef DEBUG
            if (strcmp(argv[i], "-v") == 0)
            {
                verbose = true;
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
            if (strcmp(argv[i], "-k") == 0)
            {
                keccak_metrics = true;
                continue;
            }
#endif
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
            printf("Unrecognized argument: %s\n", argv[i]);
            print_usage();
            exit(-1);
        }
    }
}

uint64_t TimeDiff(const struct timeval startTime, const struct timeval endTime)
{
    struct timeval diff;

    // Calculate the time difference
    diff.tv_sec = endTime.tv_sec - startTime.tv_sec;
    if (endTime.tv_usec >= startTime.tv_usec)
    {
        diff.tv_usec = endTime.tv_usec - startTime.tv_usec;
    }
    else if (diff.tv_sec > 0)
    {
        diff.tv_usec = 1000000 + endTime.tv_usec - startTime.tv_usec;
        diff.tv_sec--;
    }
    else
    {
        // gettimeofday() can go backwards under some circumstances: NTP, multithread...
        //cerr << "Error: TimeDiff() got startTime > endTime: startTime.tv_sec=" << startTime.tv_sec << " startTime.tv_usec=" << startTime.tv_usec << " endTime.tv_sec=" << endTime.tv_sec << " endTime.tv_usec=" << endTime.tv_usec << endl;
        return 0;
    }

    // Return the total number of us
    return diff.tv_usec + 1000000 * diff.tv_sec;
}

#ifdef DEBUG

/* Trace data structure
    [8B] Number of chunks: C

    Offset to chunk 0:
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

    Offset to chunk 1:
    …
    Offset to chunk C-1:
    …
*/
void log_trace(void)
{

    uint64_t * pOutput = (uint64_t *)TRACE_ADDR;
    printf("Version = 0x%06llx\n", pOutput[0]); // Version, e.g. v1.0.0 [8]
    printf("Exit code = %d\n", pOutput[1]); // Exit code: 0=successfully completed, 1=not completed (written at the beginning of the emulation), etc. [8]
    printf("Allocated size = %d B\n", pOutput[2]); // MT allocated size [8]
    printf("MT used size = %d B\n", pOutput[3]); // MT used size [8]

    printf("Trace content:\n");
    uint64_t * trace = (uint64_t *)MEM_TRACE_ADDRESS;
    uint64_t number_of_chunks = trace[0];
    printf("Number of chunks=%d\n", number_of_chunks);
    if (number_of_chunks > 1000000)
    {
        printf("Number of chunks is too high=%d\n", number_of_chunks);
        exit(-1);
    }
    uint64_t * chunk = trace + 1;
    for (uint64_t c=0; c<number_of_chunks; c++)
    {
        uint64_t i=0;
        printf("Chunk %d:\n", c);

        // Log current chunk start state
        printf("\tStart state:\n");
        printf("\t\tpc=0x%llx:\n", chunk[i]);
        i++;
        printf("\t\tsp=0x%llx:\n", chunk[i]);
        i++;
        printf("\t\tc=0x%llx:\n", chunk[i]);
        i++;
        printf("\t\tstep=%d:\n", chunk[i]);
        i++;
        for (uint64_t r=1; r<34; r++)
        {
            printf("\t\tregister[%d]=0x%llx:\n", r, chunk[i]);
            i++;
        }

        // Log current chunk last state
        printf("\tLast state:\n");
        printf("\t\tc=0x%llx:\n", chunk[i]);
        i++;
        
        // Log current chunk end
        printf("\tEnd:\n");
        printf("\t\tend=%d:\n", chunk[i]);
        i++;

        // Log current chunk steps
        printf("\tSteps:\n");
        printf("\t\tsteps=%d:\n", chunk[i]);
        i++;
        uint64_t mem_reads_size = chunk[i];
        printf("\t\tmem_reads_size=%d:\n", mem_reads_size);
        i++;
        if (mem_reads_size > 10000000)
        {
            printf("Mem reads size is too high=%d\n", mem_reads_size);
            exit(-1);
        }
        if (trace_trace)
        {
            for (uint64_t m=0; m<mem_reads_size; m++)
            {
                printf("\t\tchunk[%d].mem_reads[%d]=%08x:\n", c, m, chunk[i]);
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
    printf("Trace=%llx chunk=%llx size=%d\n", trace, chunk, (uint64_t)chunk - (uint64_t)trace);
}
#endif