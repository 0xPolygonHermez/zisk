#include <stdio.h>
#include <sys/mman.h>
#include <bits/mman-linux.h>

void emulator_start(void);

#define RAM_ADDR 0xa0000000
#define RAM_SIZE 0x08000000
#define SYS_ADDR RAM_ADDR
#define SYS_SIZE 0x10000
#define OUTPUT_ADDR (SYS_ADDR + SYS_SIZE)

#define ROM_ADDR 0x80000000
#define ROM_SIZE 0x08000000

int main(int argc, char *argv[])
{
    printf("Emulator C start\n");

    // Allocate ram
    void * pRam = mmap((void *)RAM_ADDR, RAM_SIZE, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
    if (pRam == NULL)
    {
        printf("Failed calling mmap(ram)\n");
        return -1;
    }
    printf("mmap(ram) returned %08x\n", pRam);

    // Allocate rom
    void * pRom = mmap((void *)ROM_ADDR, ROM_SIZE, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0);
    if (pRom == NULL)
    {
        printf("Failed calling mmap(rom)\n");
        return -1;
    }
    printf("mmap(rom) returned %08x\n", pRom);

    emulator_start();

    unsigned int * pOutput = (unsigned int *)OUTPUT_ADDR;
    unsigned int output_size = *pOutput;
    printf("Output size=%d", output_size);

    for (unsigned int i = 0; i < output_size; i++)
    {
        pOutput++;
        printf("%08x", *pOutput);
    }
    printf("Emulator C end\n");
}