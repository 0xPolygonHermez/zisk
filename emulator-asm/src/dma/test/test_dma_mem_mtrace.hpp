#ifndef __TEST_DMA_MEM_MTRACE__HPP__
#define __TEST_DMA_MEM_MTRACE__HPP__

#include <stdint.h>
#include <stdlib.h>
#include <unistd.h>
#include "test_dma_mem.hpp"

class TestDmaMemMtrace: public TestDmaMem {
protected:    
    int diff_dst_src;
    uint64_t count;
    void dump(void);
public:
    TestDmaMemMtrace(size_t max_count = 1024, bool use_src = true);
    virtual ~TestDmaMemMtrace();    
    virtual void run(void) = 0;
};

#endif