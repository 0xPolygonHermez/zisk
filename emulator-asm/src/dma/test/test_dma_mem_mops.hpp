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
    // The harness runs the recorders with r14 = TEST_STEP_LEFT and chunk_size = TEST_CHUNK_SIZE,
    // so every record of a call carries the same step field.
    static constexpr uint64_t TEST_CHUNK_SIZE = 1 << 18;
    static constexpr uint64_t TEST_STEP_LEFT = 1000;
    static constexpr uint64_t TEST_STEP_SHIFT = 38;
    static constexpr uint64_t TAG = 1ull << 63;
    static uint64_t step_field(uint64_t slot) { return (((TEST_CHUNK_SIZE - TEST_STEP_LEFT) << 2) | slot) << TEST_STEP_SHIFT; }
    // The record at word `w` has header `expected` (address | mode): a non-block read is one
    // tagged header word with the step field, a block record a tagged header and the step field
    // payload. Advances `w`.
    bool check_record(size_t &w, uint64_t expected, const char *tag);
    // The records at `w` are the aligned writes of `words` words from `addr` as value blocks
    // (header, top-bit mask, values), with the current memory contents as values. Advances `w`.
    bool check_write_records(size_t &w, uint64_t addr, size_t words, const char *tag);
};

#endif