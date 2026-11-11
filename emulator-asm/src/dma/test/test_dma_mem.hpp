#ifndef __TEST_DMA_MEM__HPP__
#define __TEST_DMA_MEM__HPP__

#include <stdint.h>
#include <stdlib.h>
#include <unistd.h>

#define EXTRA_PARAMETER_ADDR 0xC0400F00

class TestDmaMem {
protected:
    uint8_t *dst;
    uint8_t *src;
    uint64_t *aligned_dst;
    uint64_t *aligned_src;
    uint64_t *test_trace;
    uint64_t *mtrace;
    size_t max_count;
    size_t data_size;
    size_t trace_size;
    uint64_t src_offset;
    uint64_t dst_offset;
    bool use_src;
    int diff_dst_src;
    uint64_t count;
    bool low_mapped = false;
    // Moves the buffers to anonymous mappings at `base` (below 2^33, the width of a recorded
    // address): the default heap of a large test sits above it.
    void map_low(uint64_t base);
public:
    TestDmaMem(size_t max_count = 1024, bool use_src = true);
    virtual ~TestDmaMem();    
    virtual void run(void) = 0;
};

#endif