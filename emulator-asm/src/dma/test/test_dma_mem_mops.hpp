#ifndef __TEST_DMA_MEM_MTRACE_MOPS__HPP__
#define __TEST_DMA_MEM_MTRACE_MOPS__HPP__

#include <stdint.h>
#include <stdlib.h>
#include <unistd.h>
#include "test_dma_mem.hpp"

class TestDmaMemMops: public TestDmaMem {
protected:
    void dump(void);
public:
    TestDmaMemMops(size_t max_count = 1024, bool use_src = true);
    virtual ~TestDmaMemMops();    
    virtual void run(void) = 0;
    std::string decode(uint64_t value);
    uint64_t encode_read(uint32_t addr, uint8_t bytes);
    uint64_t encode_write(uint32_t addr, uint8_t bytes);
    uint64_t encode_aligned_read(uint32_t addr);
    uint64_t encode_aligned_write(uint32_t addr);
    uint64_t encode_block_read(uint32_t addr, uint32_t count);
    uint64_t encode_block_write(uint32_t addr, uint32_t count);
    uint64_t encode_aligned_block_read(uint32_t addr, uint32_t count);
    uint64_t encode_aligned_block_write(uint32_t addr, uint32_t count);
    uint64_t encode_aligned_x_read(uint32_t addr, uint32_t count);
};

#endif