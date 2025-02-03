#include <stdio.h>
#include <sys/mman.h>
#include <bits/mman-linux.h>
#include <stdbool.h>
#include <string.h>
#include <stdint.h>
#include <sys/time.h>

void emulator_start(void);

#define RAM_ADDR 0xa0000000
#define RAM_SIZE 0x08000000
#define SYS_ADDR RAM_ADDR
#define SYS_SIZE 0x10000
#define OUTPUT_ADDR (SYS_ADDR + SYS_SIZE)

#define ROM_ADDR 0x80000000
#define ROM_SIZE 0x08000000

#define INPUT_ADDR 0x90000000
#define MAX_INPUT_SIZE 0x08000000

struct timeval start_time;

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
    printf("Usage: emu <input_file> [-v verbose on] [-o output off] [-h/--help print this]\n");
}

int main(int argc, char *argv[])
{
    // Configuration
    bool verbose = false;
    bool output = true;
    char * input_file = NULL;

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

    // Call emulator assembly code
    gettimeofday(&start_time,NULL);
    emulator_start();
    struct timeval stop_time;
    gettimeofday(&stop_time,NULL);
    printf("Duration = %d us\n", TimeDiff(start_time, stop_time));

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
    printf("print_step() Counter=%d Step=%d Duration=%dus Speed=%dsteps/ms\n", print_step_counter, step, duration, speed);
    return 0;
}