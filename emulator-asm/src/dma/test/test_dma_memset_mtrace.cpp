#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <cstdio>
#include <thread>
#include <chrono>
#include <assert.h>
#include "test_dma_mem_mtrace.hpp"
#include "test_dma_memset_mtrace.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"

extern "C" {
    uint64_t test_asm_dma_memset_mtrace(uint8_t *dst, uint64_t byte, size_t count, uint64_t *trace);
}

class TestDmaMemSetMtrace: public TestDmaMemMtrace {
protected:
    uint64_t *prev_dst;
    uint64_t byte;
    bool execute_single_test(void);
public:
    TestDmaMemSetMtrace(size_t max_count = 1024);
    virtual ~TestDmaMemSetMtrace();
    void run(void);
};

TestDmaMemSetMtrace::TestDmaMemSetMtrace(size_t max_count):
    TestDmaMemMtrace(max_count,false) {
    prev_dst = (uint64_t *)malloc(data_size);
}

TestDmaMemSetMtrace::~TestDmaMemSetMtrace(void) {
    free(prev_dst);
}

void TestDmaMemSetMtrace::run(void) {
    size_t total_tests = 0;
    src_offset = 0;
    for (byte = 0; byte <= 0xFF; ++byte) {
        for (dst_offset = 0; dst_offset < 7; ++dst_offset) {
            for (count = 0; count < 1024; ++count) {
                if (!execute_single_test()) {
                    printf("\nTest is [\x1B[1;31mFAIL\x1B[0m]\n");
                    printf("---------------------------------\n");
                    size_t dst_qwords = (dst_offset + count + 7) >> 3;
                    for (size_t index = 0; index < dst_qwords; ++index) {
                        printf("prev_dst64[%ld] 0x%016lX\n", index, prev_dst[index]);
                    }                    
                    dump();
                    return;
                }
                ++total_tests;
            }
        }
    }
    printf("\nAll %ld tests are [\x1B[1;32mOK\x1B[0m]\n", total_tests);
}

bool TestDmaMemSetMtrace::execute_single_test(void) {
    memset(test_trace, 0, trace_size);
    fill_pattern(dst, data_size, 18219046755);
    uint8_t *p_dst = dst + dst_offset;

    memcpy(prev_dst, dst, data_size);
    printf("\rTEST byte:0x%02lX dst_offset:%ld count:%4ld", 
            byte, dst_offset, count);
    fflush(stdout);
    uint64_t res = test_asm_dma_memset_mtrace(p_dst, byte, count, test_trace);
    size_t trace_count = test_trace[0];
    if (trace_count < 1) {
        printf("\nERROR: invalid trace_count %ld\n", trace_count);
        return false;
    }
    uint64_t _dst = (uint64_t)dst + dst_offset;
    if (res != _dst) {
        printf("\nERROR: invalid result expected:0x%08lX found:0x%08lX\n", _dst, res);
        return false;                   
    }        
    uint64_t encode = calculate_encode_memset((uint64_t)p_dst, count, byte);
    if (mtrace[0] != encode) {
        printf("\nERROR: invalid encoded\n");
        print_encode_mismatch(encode, mtrace[0]);
        return false;                   
    }
    size_t index = 1;
    size_t pre_count = (dst_offset > 0 && count > 0) ? 8 - dst_offset : 0;
    if (pre_count > count) {
        pre_count = count;
    }
    if (pre_count > 0) {
        if (mtrace[index] != prev_dst[0]) {
            printf("\nERROR: pre write pre-value expected: dst[0]:0x%016lX found: mtrace[%ld]:%016lX\n", prev_dst[0], index, mtrace[index]);
            return false;
        }
        ++index;        
    }

    size_t post_count = (count - pre_count) & 0x07;
    if (post_count > 0) {
        size_t last_dst_index = (dst_offset + count - 1) >> 3;
        if (mtrace[index] != prev_dst[last_dst_index]) {
            printf("\nERROR: post write pre-value expected: dst[%ld]:0x%016lX vs found mtrace[%ld]:0x%016lX\n", last_dst_index, prev_dst[last_dst_index], index, mtrace[index]);
            return false;
        }
        ++index;        
    }
    if (trace_count != index) {
        printf("ERROR: invalid mtrace len expected:%ld vs found:%ld\n", index, trace_count);
        size_t _count = index > trace_count ? index + 1: trace_count + 1;
        for (size_t i = 0; i <= _count; ++i) {
            printf("mtrace[%ld] 0x%016lX\n", i, mtrace[i]);
        }
        return false;
    }
    return true;
}

void test_dma_memset_mtrace() {
    printf("\x1B[1;34mTEST DMA MEMSET MTRACE =================================================\x1B[0m\n");
    TestDmaMemSetMtrace test(1024);
    test.run();
}
