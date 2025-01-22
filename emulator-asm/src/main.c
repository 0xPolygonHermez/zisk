#include <stdio.h>
#include <sys/mman.h>
#include <bits/mman-linux.h>
#include <stdbool.h>
#include <string.h>
#include <stdint.h>

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
    emulator_start();

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