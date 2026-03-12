#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <cstdio>
#include <thread>
#include <chrono>
#include <assert.h>
#include "test_dma_mem_mtrace.hpp"
#include "test_dma_memcpy_mtrace.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"

#define BUFFER_SIZE (sizeof(uint64_t) * 2 * 1024)

extern "C" {
    uint64_t test_asm_dma_memcpy_mtrace(uint8_t *dst, uint8_t *src, size_t count, uint64_t *trace);
}

class TestDmaMemCpyMtrace: public TestDmaMemMtrace {
protected:
    uint64_t *prev_dst;
    uint64_t *check_dst;
    bool execute_single_test(void);
public:
    TestDmaMemCpyMtrace(size_t max_count = 1024);
    virtual ~TestDmaMemCpyMtrace();
    void run(void);
};

TestDmaMemCpyMtrace::TestDmaMemCpyMtrace(size_t max_count):
    TestDmaMemMtrace(max_count) {
    prev_dst = (uint64_t *)malloc(data_size);
    check_dst = (uint64_t *)malloc(data_size);
}

TestDmaMemCpyMtrace::~TestDmaMemCpyMtrace(void) {
    free(prev_dst);
    free(check_dst);
}

void TestDmaMemCpyMtrace::run(void) {
    fill_pattern(src,data_size, 3013102105130209);
    size_t total_tests = 0;
    for (dst_offset = 0; dst_offset < 7; ++dst_offset) {
        for (src_offset = 0; src_offset < 7; ++src_offset) {
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

bool TestDmaMemCpyMtrace::execute_single_test(void) {
    memset(test_trace, 0, trace_size);
    fill_pattern(dst, data_size, 1821904675);
    uint8_t *p_dst = dst + dst_offset;
    uint8_t *p_src = src + src_offset;

    memcpy(prev_dst, dst, data_size);
    printf("\rTEST dst_offset:%ld src_offset:%ld count:%4ld", 
            dst_offset, src_offset, count);
    fflush(stdout);
    uint64_t res = test_asm_dma_memcpy_mtrace(p_dst, p_src, count, test_trace);
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
    uint64_t encode = calculate_encode((uint64_t)p_dst, (uint64_t)p_src, count);
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
    size_t src_qwords = count > 0 ? (src_offset + count + 7) >> 3 : 0;
    for (size_t i_src = 0; i_src < src_qwords; ++i_src) {
        if (mtrace[index] != aligned_src[i_src]) {
            printf("ERROR: src value expected: src[%ld]:0x%016lX vs found mtrace[%ld]:0x%016lX\n", i_src, aligned_src[i_src], index, mtrace[index]);
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
    memcpy((uint8_t *)prev_dst + dst_offset, src + src_offset, count);
    if (memcmp(prev_dst, dst, data_size) != 0) {
        int errors = 0;
        uint8_t *_dst = (uint8_t *)prev_dst;
        for (size_t i = 0; i < data_size; ++i) {
            if (_dst[i] == src[i]) continue;
            printf("[%ld] 0x%02X 0x%02X NO MATCH\n", i, _dst[i], src[i]);
            ++errors;
            if (errors > 16) {
                printf(".... and more\n");
                break;
            }
        }
        printf("\nERROR: memcpy operation\n");
        return false;
    }    
    return true;
}

void test_dma_memcpy_mtrace() {
    printf("\x1B[1;34mTEST DMA MEMCPY MTRACE =================================================\x1B[0m\n");
    TestDmaMemCpyMtrace test(1024);
    test.run();
}
