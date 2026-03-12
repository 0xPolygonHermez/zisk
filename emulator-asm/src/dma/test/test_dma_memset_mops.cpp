#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <cstdio>
#include <thread>
#include <chrono>
#include <string>
#include <assert.h>
#include "test_dma_mem_mops.hpp"
#include "test_dma_memset_mops.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"

extern "C" {
    uint64_t test_asm_dma_memset_mops(uint8_t *dst, uint64_t byte, size_t count, uint64_t *trace);
}

class TestDmaMemSetMops: public TestDmaMemMops {
protected:
    uint64_t *prev_dst;
    uint64_t byte;
    bool execute_single_test(void);
    bool check_mop(size_t index, uint64_t expected, const char *tag);
public:
    TestDmaMemSetMops(size_t max_count = 1024);
    virtual ~TestDmaMemSetMops();
    void run(void);
};

TestDmaMemSetMops::TestDmaMemSetMops(size_t max_count):
    TestDmaMemMops(max_count, false) {
    prev_dst = (uint64_t *)malloc(data_size);
}

TestDmaMemSetMops::~TestDmaMemSetMops(void) {
    free(prev_dst);
}

void TestDmaMemSetMops::run(void) {
    printf("DST:0x%08lX\n", (uint64_t)dst);
    size_t total_tests = 0;
    src_offset = 0;
    for (byte = 0; byte <= 0xFF; ++byte) {
        for (dst_offset = 0; dst_offset < 7; ++dst_offset) {
            for (count = 0; count < 1024; ++count) {
                if (!execute_single_test()) {
                    printf("\nTest is [\x1B[1;31mFAIL\x1B[0m]\n");
                    dump();
                    return;
                }
                ++total_tests;
            }
        }
    }
    printf("\nAll %ld tests are [\x1B[1;32mOK\x1B[0m]\n", total_tests);
}

bool TestDmaMemSetMops::check_mop(size_t index, uint64_t expected, const char *tag) {
    if (mtrace[index] != expected) {
        printf("\nERROR: %s expected: 0x%016lX (%s) found: mtrace[%ld]:%016lX (%s)\n", tag, expected, 
                decode(expected).c_str(), index, mtrace[index], decode(mtrace[index]).c_str());
        return false;
    }
    return true;
}
bool TestDmaMemSetMops::execute_single_test(void) {
    memset(test_trace, 0, trace_size);
    fill_pattern(dst, data_size, 1821904675);
    uint8_t *p_dst = dst + dst_offset;

    memcpy(prev_dst, dst, data_size);
    printf("\rTEST byte:0x%02lX dst_offset:%ld count:%4ld", 
            byte, dst_offset, count);
    fflush(stdout);
    uint64_t res = test_asm_dma_memset_mops(p_dst, byte, count, test_trace);
    size_t trace_count = test_trace[0];
    uint64_t _dst = (uint64_t)dst + dst_offset;
    if (res != _dst) {
        printf("\nERROR: invalid result expected:0x%08lX found:0x%08lX\n", _dst, res);
        return false;                   
    }    
    size_t index = 0;
    size_t pre_count = (dst_offset > 0 && count > 0) ? 8 - dst_offset : 0;
    if (pre_count > count) {
        pre_count = count;
    }
    if (pre_count > 0) {
        if (!check_mop(index, encode_aligned_read((uint64_t)dst), "PRE pre write") ) {
            return false;
        }
        index += 1;
    }
    size_t loop_count = (count - pre_count) >> 3;
    size_t post_count = (count - pre_count) & 0x07;
    if (post_count > 0) {
        uint64_t dst_post = ((uint64_t)dst + dst_offset + pre_count + loop_count * 8) & ~0x07;
        if (!check_mop(index, encode_aligned_read((uint64_t)dst_post), "POST pre write")) {
            return false;
        }
        index += 1;
    }
    if (count > 0) {
        size_t dst_qwords = (dst_offset + count + 7) >> 3;
        if (!check_mop(index, encode_aligned_block_write((uint64_t)dst, dst_qwords), "dst write")) {
            return false;
        }
        ++index;
    }
    if (trace_count != index) {
        printf("ERROR: invalid mtrace len expected:%ld vs found:%ld\n", index, trace_count);
        return false;
    }
    memset((uint8_t *)prev_dst + dst_offset, byte, count);
    if (memcmp(prev_dst, dst, data_size) != 0) {
        printf("\nERROR: memset operation\n");
        return false;
    }
    return true;
}

void test_dma_memset_mops() {
    printf("\x1B[1;34mTEST DMA MEMSET MOPS =================================================\x1B[0m\n");
    TestDmaMemSetMops test(1024);
    test.run();
}
