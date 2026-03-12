#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <cstdio>
#include <thread>
#include <chrono>
#include <assert.h>
#include "test_dma_mem_mtrace.hpp"
#include "test_dma_memcmp_mtrace.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"

extern "C" {
    size_t test_asm_dma_memcmp_mtrace(uint8_t *dst, uint8_t *src, size_t count, uint64_t *trace);
}

class TestDmaMemCmpMtrace: public TestDmaMemMtrace {
protected:
    int diff_dst_src;
    uint64_t bus_count;
    bool execute_single_test(void);
public:
    TestDmaMemCmpMtrace(size_t max_count = 1024);
    void run(void);
};

TestDmaMemCmpMtrace::TestDmaMemCmpMtrace(size_t max_count):
    TestDmaMemMtrace(max_count) {
}


void TestDmaMemCmpMtrace::run(void) {
    fill_pattern(src,data_size, 3013102105130209);
    size_t total_tests = 0;
    for (uint64_t icase = 0; icase < 3; ++icase) {
        diff_dst_src = icase == 0 ? 0 : (60 * icase - 180 * (icase - 1));
        for (dst_offset = 0; dst_offset < 7; ++dst_offset) {
            for (src_offset = 0; src_offset < 7; ++src_offset) {
                for (count = 0; count < 1024; ++count) {
                    for (uint64_t i_count_case = 0; i_count_case < 3; ++i_count_case) {
                        if (i_count_case > 0) {
                            bus_count = count + 1 + (i_count_case - 1) * (dst_offset + src_offset);
                        }
                        if (!execute_single_test()) {
                            printf("\nTest is [\x1B[1;31mFAIL\x1B[0m]\n");
                            dump();
                            return;
                        }
                        ++total_tests;
                    }
                }
            }
        }
    }
    printf("\nAll %ld tests are [\x1B[1;32mOK\x1B[0m]\n", total_tests);
}

bool TestDmaMemCmpMtrace::execute_single_test(void) {
    memset(test_trace, 0, trace_size);
    fill_pattern(dst, data_size, 1821904675);
    bus_count = count;
    uint8_t *p_dst = dst + dst_offset;
    uint8_t *p_src = src + src_offset;

    int cmp_res = create_memcmp_data(p_dst, p_src, count, diff_dst_src);
    printf("\rTEST dst_offset:%ld src_offset:%ld count:%4ld (bus_count:%4ld) cmp_res:%4d (diff:%4d)", 
            dst_offset, src_offset, count, bus_count, cmp_res, diff_dst_src);
    fflush(stdout);
    int res = test_asm_dma_memcmp_mtrace(p_dst, p_src, bus_count, test_trace);
    size_t trace_count = test_trace[0];
    if (trace_count < 2) {
        printf("\nERROR: invalid trace_count %ld\n", trace_count);
        return false;
    }
    if (res != cmp_res) {
        uint8_t byte_dst = dst[dst_offset + count - 1];
        uint8_t byte_src = src[src_offset + count - 1];
        printf("\nERROR: invalid result expected:%d found:%d DST:0x%02X SRC:0x%02X\n", 
                  cmp_res, res, byte_dst, byte_src);
        return false;                   
    }
    uint64_t encode = calculate_encode_memcmp((uint64_t)p_dst, (uint64_t)p_src, count, res);
    if (mtrace[0] != encode) {
        printf("\nERROR: invalid encoded\n");
        print_encode_mismatch(encode, mtrace[0]);
        return false;                   
    }
    if (mtrace[1] != bus_count) {
        printf("ERROR: invalid bus_count expected:%ld found:%ld\n", bus_count, mtrace[1]);
        return false;
    }
    size_t index = 2;
    size_t pre_count = (dst_offset > 0 && count > 0) ? 8 - dst_offset : 0;
    if (pre_count > count) {
        pre_count = count;
    }
    if (pre_count > 0) {
        if (mtrace[index] != aligned_dst[0]) {
            printf("ERROR: pre write pre-value expected: dst[0]:0x%016lX found: mtrace[%ld]:%016lX\n", aligned_dst[0], index, mtrace[index]);
            return false;
        }
        ++index;        
    }

    size_t loop_count = (count - pre_count) >> 3;
    size_t post_count = (count - pre_count) & 0x07;
    if (loop_count > 0 && post_count == 0 && res != 0) {
        loop_count -= 1;
        post_count = 8;
    }
    if (post_count > 0) {
        size_t last_dst_index = (dst_offset + count - 1) >> 3;
        if (mtrace[index] != aligned_dst[last_dst_index]) {
            printf("ERROR: post write pre-value expected: dst[%ld]:0x%016lX vs found mtrace[%ld]:0x%016lX\n", last_dst_index, aligned_dst[last_dst_index], index, mtrace[index]);
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
    
    return true;
}

void test_dma_memcmp_mtrace() {
    printf("\x1B[1;34mTEST DMA MEMCMP MTRACE =================================================\x1B[0m\n");
    TestDmaMemCmpMtrace test(1024);
    test.run();
}