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
#include "test_dma_memcmp_mops.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"

extern "C" {
    size_t test_asm_dma_memcmp_mops(uint8_t *dst, uint8_t *src, size_t count, uint64_t *trace);
}

class TestDmaMemCmpMops: public TestDmaMemMops {
protected:
    int diff_dst_src;
    uint64_t bus_count;
    bool execute_single_test(void);
    bool check_mop(size_t index, uint64_t expected, const char *tag);
public:
    TestDmaMemCmpMops(size_t max_count = 1024);
    void run(void);
};

TestDmaMemCmpMops::TestDmaMemCmpMops(size_t max_count):
    TestDmaMemMops(max_count) {
}


void TestDmaMemCmpMops::run(void) {
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

bool TestDmaMemCmpMops::check_mop(size_t index, uint64_t expected, const char *tag) {
    if (mtrace[index] != expected) {
        printf("\nERROR: %s expected: 0x%016lX (%s) found: mtrace[%ld]:%016lX (%s)\n", tag, expected, 
                decode(expected).c_str(), index, mtrace[index], decode(mtrace[index]).c_str());
        return false;
    }
    return true;
}
bool TestDmaMemCmpMops::execute_single_test(void) {
    memset(test_trace, 0, trace_size);
    fill_pattern(dst, data_size, 1821904675);
    uint8_t *p_dst = dst + dst_offset;
    uint8_t *p_src = src + src_offset;

    int cmp_res = create_memcmp_data(p_dst, p_src, count, diff_dst_src);
    printf("\rTEST dst_offset:%ld src_offset:%ld count:%4ld (bus_count:%4ld) cmp_res:%4d (diff:%4d)", 
            dst_offset, src_offset, count, bus_count, cmp_res, diff_dst_src);
    fflush(stdout);
    int res = test_asm_dma_memcmp_mops(p_dst, p_src, count, test_trace);
    size_t trace_count = test_trace[0];
    if (trace_count < 1) {
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
    if (mtrace[0] != encode_aligned_read(EXTRA_PARAMETER_ADDR)) {
        printf("\nERROR: not found valid param read\n");
        return false;
    } 
    // uint64_t encode = calculate_encode((uint64_t)p_dst, (uint64_t)p_src, count);
    size_t index = 1;
    size_t pre_count = (dst_offset > 0 && count > 0) ? 8 - dst_offset : 0;
    if (pre_count > count) {
        pre_count = count;
    }
    if (pre_count > 0) {
        size_t src_blocks = 1 + ((src_offset + pre_count) > 8);
        if (!check_mop(index, encode_aligned_read((uint64_t)dst), "PRE pre write") ||
            !check_mop(index + 1, encode_aligned_x_read((uint64_t)src, src_blocks), "PRE src read")) {
                return false;
        }
        index += 2;
    }
    size_t loop_count = (count - pre_count) >> 3;
    size_t post_count = (count - pre_count) & 0x07;
    if (loop_count > 0 && post_count == 0 && res != 0) {
        loop_count -= 1;
        post_count = 8;
    }    
    if (post_count > 0) {
        uint64_t src_post = ((uint64_t)src + src_offset + pre_count + loop_count * 8) & ~0x07;
        uint64_t dst_post = ((uint64_t)dst + dst_offset + pre_count + loop_count * 8) & ~0x07;
        size_t src_blocks = 1 + ((((src_offset + pre_count) & 0x07) + post_count) > 8);
        if (!check_mop(index, encode_aligned_read((uint64_t)dst_post), "POST pre write") ||
            !check_mop(index + 1, encode_aligned_x_read((uint64_t)src_post, src_blocks), "POST src read")) {
                return false;
        }
        index += 2;
    }
    if (loop_count > 0) {
        uint64_t src_loop = ((uint64_t)src + src_offset + pre_count) & ~0x07;
        uint64_t dst_loop = pre_count > 0 ? (uint64_t)dst + 8 : (uint64_t)dst;
        size_t src_count = dst_offset == src_offset ? loop_count : (loop_count + 1);

        if (!check_mop(index, encode_aligned_block_read(src_loop, src_count), "LOOP src read")) {
            return false;
        }
        if (!check_mop(index+1, encode_aligned_block_read(dst_loop, loop_count), "LOOP dst read (cmp)")) {
            return false;
        }
        index += 2;
    }
    if (trace_count != index) {
        printf("ERROR: invalid mtrace len expected:%ld vs found:%ld\n", index, trace_count);
        return false;
    }
    return true;
}


void test_dma_memcmp_mops() {
    printf("\x1B[1;34mTEST DMA MEMCMP MOPS =================================================\x1B[0m\n");
    TestDmaMemCmpMops test(1024);
    test.run();    
}
