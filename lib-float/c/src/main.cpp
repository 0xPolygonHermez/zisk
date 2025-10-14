#include "float/float.h"

#ifdef ZISK_GCC
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h> // For mmap
#include <unistd.h>   // For sysconf
#endif

#ifdef __cplusplus
extern "C" {
#endif

#ifdef ZISK_GCC
#define TARGET_ADDRESS ((void *)0xa0000000)
#define MAP_SIZE (256 * 1024 * 1024UL) // 256 MB
#endif

int _zisk_main(int argc, char *argv[])
{
    // f3 = f1 + f2
    //fregs[1] = F64_ONE;
    //fregs[2] = F64_ONE;
    //uint64_t inst = 0x022081D3; // fadd.d f1, f2, f3

    // fregs[31] = (uint64_t)0x807fffff;
    // fregs[30] = 0x80444444;
    fregs[31] = 0x8010000000000000;
    fregs[30] = 0x88100000001fffff;
    //uint64_t inst = 0xa3ffa953; // feq.d x18, f31, f31
    //uint64_t inst = 0x09EF8ED3; // fsub.d f1, f31, f30
    uint64_t inst = 0x03EF9ED3;
    *(uint64_t *)FREG_INST = inst;
    _zisk_float();
    uint64_t reg = fregs_x[18];
    return 0;
}

#ifdef ZISK_GCC

int main(int argc, char *argv[]) {
    // Attempt to create the memory mapping
    void *mapped_region = mmap(
        TARGET_ADDRESS,        // Requested starting address
        MAP_SIZE,              // Length of the mapping (256 MB)
        PROT_READ | PROT_WRITE, // Memory protection: Readable & Writable
        MAP_ANONYMOUS |        // Mapping is not file-backed...
        MAP_PRIVATE |          // ... and is private to this process
        MAP_FIXED,             // Use the requested address exactly (DANGEROUS)
        -1,                    // File descriptor (ignored for MAP_ANONYMOUS)
        0                      // Offset (ignored for MAP_ANONYMOUS)
    );

    // Check if mmap succeeded
    if (mapped_region == MAP_FAILED) {
        perror("mmap failed");
        exit(EXIT_FAILURE);
    }

    // Since we used MAP_FIXED, the returned address should be exactly
    // what we requested. Let's verify.
    if (mapped_region != TARGET_ADDRESS) {
        fprintf(stderr, "Error: Mapping ended up at %p, not %p\n",
                mapped_region, TARGET_ADDRESS);
        munmap(mapped_region, MAP_SIZE); // Clean up
        exit(EXIT_FAILURE);
    }

    printf("Successfully mapped 256 MB at address %p\n", mapped_region);

    int result_value = _zisk_main(argc, argv);

    for (int i = 0; i < 32; i++) {
        printf("fregs[%2d] = 0x%016llx\n", i, fregs[i]);
    }

    for (int i = 0; i < 32; i++) {
        printf("fregs_x[%2d] = 0x%016llx\n", i, fregs_x[i]);
    }

    printf("fcsr = 0x%016x\n", fcsr);

    // It's good practice to unmap the memory when you're done, though
    // the OS will automatically clean up on program exit.
    printf("Unmapping the region...\n");
    if (munmap(mapped_region, MAP_SIZE) == -1) {
        perror("munmap failed");
    }

    return result_value;
}
#endif // ZISK_GCC

#ifdef __cplusplus
} // extern "C"
#endif