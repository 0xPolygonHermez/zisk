#include <stdio.h>
#include <sys/mman.h>
#include <bits/mman-linux.h>
#include <stdbool.h>
#include <string.h>
#include <stdint.h>
#include <sys/time.h>

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

#define TRACE_ADDR 0xc0000000
#define TRACE_SIZE 0x40000000 // 1GB

struct timeval start_time;

extern uint64_t MEM_STEP;
extern uint64_t MEM_TRACE_ADDRESS;
extern uint64_t MEM_CHUNK_ADDRESS;
extern uint64_t MEM_CHUNK_START_STEP;

struct timeval keccak_start, keccak_stop;
uint64_t keccak_counter = 0;
uint64_t keccak_duration = 0;

extern void keccakf1600_generic(uint64_t state[25]);

extern void zisk_keccakf(uint64_t state[25]);

#define CHUNK_SIZE 1024*1024
uint64_t chunk_size = CHUNK_SIZE;
uint64_t chunk_size_mask = CHUNK_SIZE - 1;
uint64_t trace_address = TRACE_ADDR;

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


void print_usage (void)
{
    printf("Usage: emu <input_file> [-v verbose on] [-o output off]  [-t trace on] [-h/--help print this]\n");
}

// Configuration
bool verbose = false;
bool output = true;
bool trace = false;
bool trace_trace = false;
bool metrics = false;
bool keccak_metrics = false;
char * input_file = NULL;

int main(int argc, char *argv[])
{
    /*
    {
        uint64_t data[25] = {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0};
        keccakf1600_generic((uint64_t *)data);
        for (uint64_t i=0; i<25; i++)
        {
            printf("data1[%d]=%016llx\n", i, data[i]);
        }
    }
    {
        uint64_t data[25] = {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0};
        zisk_keccakf((uint64_t *)data);
        for (uint64_t i=0; i<25; i++)
        {
            printf("data2[%d]=%016llx\n", i, data[i]);
        }
    }
    {
        uint64_t data[25] = {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0};
        gettimeofday(&keccak_start, NULL);
        for (uint64_t i=0; i<1000000; i++)
        {
            keccakf1600_generic((uint64_t *)data);
        }
        gettimeofday(&keccak_stop, NULL);
        keccak_duration += TimeDiff(keccak_start, keccak_stop);
        keccak_counter = 1000000;
        uint64_t single_keccak_duration_ns = keccak_counter == 0 ? 0 : (keccak_duration * 1000) / keccak_counter;
        printf("Keccak1 counter = %d, duration = %d us, single keccak duration = %d ns\n", keccak_counter, keccak_duration, single_keccak_duration_ns);
    }
    {
        uint64_t data[25] = {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0};
        gettimeofday(&keccak_start, NULL);
        for (uint64_t i=0; i<1000000; i++)
        {
            zisk_keccakf((uint64_t *)data);
        }
        gettimeofday(&keccak_stop, NULL);
        keccak_duration += TimeDiff(keccak_start, keccak_stop);
        keccak_counter = 1000000;
        uint64_t single_keccak_duration_ns = keccak_counter == 0 ? 0 : (keccak_duration * 1000) / keccak_counter;
        printf("Keccak2 counter = %d, duration = %d us, single keccak duration = %d ns\n", keccak_counter, keccak_duration, single_keccak_duration_ns);
    }
    return 0;*/

    // Parse arguments
    if (argc > 1)
    {
        for (int i = 1; i < argc; i++)
        {
            if (strcmp(argv[i], "-v") == 0)
            {
                verbose = true;
                continue;
            }
            if (strcmp(argv[i], "-o") == 0)
            {
                output = false;
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
            if (strcmp(argv[i], "-m") == 0)
            {
                metrics = true;
                continue;
            }
            if (strcmp(argv[i], "-k") == 0)
            {
                keccak_metrics = true;
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
            if (input_file == NULL)
            {
                input_file = argv[i];
                continue;
            }
            printf("Unrecognized argument: %s\n", argv[i]);
            print_usage();
            return -1;
        }
    }
    if (verbose) printf("Emulator C start\n");

    // Allocate ram
    void * pRam = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
    if (pRam == NULL)
    {
        printf("Failed calling mmap(ram)\n");
        return -1;
    }

    if (verbose) printf("mmap(ram) returned %08x\n", pRam);

    // Allocate rom
    void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
    if (pRom == NULL)
    {
        printf("Failed calling mmap(rom)\n");
        return -1;
    }
    if (verbose) printf("mmap(rom) returned %08x\n", pRom);

    // Allocate input

    FILE *input_fp;
    long input_file_size = 0;
    if (input_file != NULL)
    {
        input_fp = fopen(input_file, "r");
        if (input_fp == NULL)
        {
            printf("Failed calling fopen(%s); does it exist?\n", input_file);
            return -1;
        }

        if (fseek(input_fp, 0, SEEK_END) == -1)
        {
            printf("Failed calling fseek(%s)\n", input_file);
            return -1;
        }

        input_file_size = ftell(input_fp);
        if (input_file_size == -1)
        {
            printf("Failed calling ftell(%s)\n", input_file);
            return -1;
        }
        // Go back to the first byte
        if (fseek(input_fp, 0, SEEK_SET) == -1)
        {
            printf("Failed calling fseek(%s, 0)\n", input_file);
            return -1;
        }

        // Check the input data size is inside the proper range
        if (input_file_size > (MAX_INPUT_SIZE - 8))
        {
            printf("Size of input file (%s) is too long (%d)\n", input_file, input_file_size);
            return -1;
        }
    }

    long input_size = ((input_file_size + 8 + 1) >> 3) << 3;
    void * pInput = mmap((void *)INPUT_ADDR, input_size, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
    if (pInput == NULL)
    {
        printf("Failed calling mmap(input)\n");
        return -1;
    }
    if (verbose) printf("mmap(input) returned %08x\n", pInput);
    *(uint64_t *)INPUT_ADDR = (uint64_t)input_file_size;

    if (input_file != NULL)
    {
        size_t input_read = fread((void *)(INPUT_ADDR + 8), 1, input_file_size, input_fp);
        if (input_read != input_file_size)
        {
            printf("Input read (%d) != input file size (%d)\n", input_read, input_file_size);
            return -1;
        }
    }

    // Allocate trace
    void * pTrace = mmap((void *)TRACE_ADDR, TRACE_SIZE, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
    if (pTrace == NULL)
    {
        printf("Failed calling mmap(pTrace)\n");
        return -1;
    }

    if (verbose) printf("mmap(trace) returned %08x\n", pTrace);

    // Call emulator assembly code
    gettimeofday(&start_time,NULL);
    emulator_start();
    struct timeval stop_time;
    gettimeofday(&stop_time,NULL);
    if (keccak_metrics || metrics || verbose)
    {
        uint64_t duration = TimeDiff(start_time, stop_time);
        uint64_t steps = MEM_STEP;
        uint64_t step_duration_ns = steps == 0 ? 0 : (duration * 1000) / steps;
        uint64_t step_tp_sec = duration == 0 ? 0 : steps * 1000000 / duration;
        uint64_t mem_trace_address = MEM_TRACE_ADDRESS;
        uint64_t mem_chunk_address = MEM_CHUNK_ADDRESS;
        uint64_t trace_size = mem_chunk_address - mem_trace_address;
        uint64_t trace_size_percentage = (trace_size * 100) / TRACE_SIZE;
        printf("Duration = %d us, Keccak counter = %d, steps = %d, step duration = %d ns, tp = %d steps/s, trace size = %d B(%d%%)\n", duration, keccak_counter, steps, step_duration_ns, step_tp_sec, trace_size, trace_size_percentage);
        if (keccak_metrics)
        {
            uint64_t keccak_percentage = duration == 0 ? 0 : (keccak_duration * 100) / duration;
            uint64_t single_keccak_duration_ns = keccak_counter == 0 ? 0 : (keccak_duration * 1000) / keccak_counter;
            printf("Keccak counter = %d, duration = %d us, single keccak duration = %d ns, percentage = %d \n", keccak_counter, keccak_duration, single_keccak_duration_ns, keccak_percentage);
        }
    }

    // Log output
    if (output)
    {
        unsigned int * pOutput = (unsigned int *)OUTPUT_ADDR;
        unsigned int output_size = *pOutput;
        if (verbose) printf("Output size=%d\n", output_size);

        for (unsigned int i = 0; i < output_size; i++)
        {
            pOutput++;
            printf("%08x\n", *pOutput);
        }
    }

    // Log trace
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
        [8B] pc
        [8B] sp
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
    if (trace)
    {
        printf("Trace content:\n");
        uint64_t * trace = (uint64_t *)pTrace;
        uint64_t number_of_chunks = trace[0];
        printf("Number of chunks=%d\n", number_of_chunks);
        if (number_of_chunks > 1000000)
        {
            printf("Number of chunks is too high=%d\n", number_of_chunks);
            return -1;
        }
        uint64_t * chunk = trace + 1;
        for (uint64_t c=0; c<number_of_chunks; c++)
        {
            uint64_t i=0;
            printf("Chunk %d:\n", c);

            // Log current chunk start state
            printf("\tStart state:\n");
            printf("\t\tpc=0x%08x:\n", chunk[i]);
            i++;
            printf("\t\tsp=0x%08x:\n", chunk[i]);
            i++;
            printf("\t\tc=0x%08x:\n", chunk[i]);
            i++;
            printf("\t\tstep=%d:\n", chunk[i]);
            i++;
            for (uint64_t r=1; r<34; r++)
            {
                printf("\t\tregister[%d]=0x%08x:\n", r, chunk[i]);
                i++;
            }

            // Log current chunk last state
            printf("\tLast state:\n");
            printf("\t\tpc=0x%08x:\n", chunk[i]);
            i++;
            printf("\t\tsp=0x%08x:\n", chunk[i]);
            i++;
            printf("\t\tc=0x%08x:\n", chunk[i]);
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
                return -1;
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
        printf("Trace=%08x chunk=%08x size=%d\n", trace, chunk, chunk - trace);
    }

    if (verbose) printf("Emulator C end\n");
}

extern int _print_abcflag(uint64_t a, uint64_t b, uint64_t c, uint64_t flag)
{
    printf("a=%08llx b=%08llx c=%08llx flag=%08llx\n", a, b, c, flag);
    fflush(stdout);
    return 0;
}

extern int _print_char(uint64_t param)
{
    char c = param;
    printf("%c", c);
    return 0;
}

uint64_t print_step_counter = 0;
extern int _print_step(uint64_t step)
{
    print_step_counter++;
    struct timeval stop_time;
    gettimeofday(&stop_time,NULL);
    uint64_t duration = TimeDiff(start_time, stop_time);
    uint64_t duration_s = duration/1000;
    if (duration_s == 0) duration_s = 1;
    uint64_t speed = step / duration_s;
    if (verbose) printf("print_step() Counter=%d Step=%d Duration=%dus Speed=%dsteps/ms\n", print_step_counter, step, duration, speed);
    return 0;
}

extern int _opcode_keccak(uint64_t address)
{
    if (keccak_metrics || verbose) gettimeofday(&keccak_start, NULL);
    //if (verbose) printf("opcode_keccak() calling KeccakF1600() counter=%d step=%08llx address=%08llx\n", keccak_counter, /**(uint64_t *)*/MEM_STEP, address);
    keccakf1600_generic((uint64_t *)address);
    //zisk_keccakf((uint64_t *)address);
    //if (verbose) printf("opcode_keccak() called KeccakF1600()\n");
    keccak_counter++;
    if (keccak_metrics || verbose)
    {
        gettimeofday(&keccak_stop, NULL);
        keccak_duration += TimeDiff(keccak_start, keccak_stop);
    }
    return 0;
}