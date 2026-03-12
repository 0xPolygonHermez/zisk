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
#include "test_dma_inputcpy_mops.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"
#include "test_mock.hpp"

extern "C" {
    uint64_t test_asm_dma_inputcpy_mops(uint8_t *dst, uint8_t *src, size_t count, uint64_t *trace);
}

class TestDmaInputCpyMops: public TestDmaMemMops {
protected:
    uint64_t *prev_dst;
    bool execute_single_test(void);
    bool check_mop(size_t index, uint64_t expected, const char *tag);
public:
    TestDmaInputCpyMops(size_t max_count = 1024);
    virtual ~TestDmaInputCpyMops();
    void run(void);
};

TestDmaInputCpyMops::TestDmaInputCpyMops(size_t max_count):
    TestDmaMemMops(max_count, false) {
    prev_dst = (uint64_t *)malloc(data_size);
}

TestDmaInputCpyMops::~TestDmaInputCpyMops(void) {
    free(prev_dst);
}

void TestDmaInputCpyMops::run(void) {
    fill_pattern((uint8_t *)(fcall_ctx + FCALL_RESULT), FCALL_RESULT_LENGTH, 3013102105130209);
    printf("DST:0x%08lX\n", (uint64_t)dst);
    size_t total_tests = 0;
    src_offset = 0;
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
    printf("\nAll %ld tests are [\x1B[1;32mOK\x1B[0m]\n", total_tests);
}

bool TestDmaInputCpyMops::check_mop(size_t index, uint64_t expected, const char *tag) {
    if (mtrace[index] != expected) {
        printf("\nERROR: %s expected: 0x%016lX (%s) found: mtrace[%ld]:%016lX (%s)\n", tag, expected, 
                decode(expected).c_str(), index, mtrace[index], decode(mtrace[index]).c_str());
        return false;
    }
    return true;
}
bool TestDmaInputCpyMops::execute_single_test(void) {
    fill_pattern((uint8_t *)(fcall_ctx + FCALL_RESULT), FCALL_RESULT_LENGTH, 15436 + dst_offset + count);
    fcall_ctx[FCALL_RESULT_GOT] = 1;
    fill_pattern(dst, data_size, 1821904675 + dst_offset + count);
    uint8_t *p_dst = dst + dst_offset;

    memcpy(prev_dst, dst, data_size);
    printf("\rTEST dst_offset:%ld count:%4ld", dst_offset, count);
    fflush(stdout);
    uint64_t res = test_asm_dma_inputcpy_mops(p_dst, 0, count, test_trace);
    size_t trace_count = test_trace[0];
    uint64_t _dst = (uint64_t)dst + dst_offset;
    if (res != _dst) {
        printf("\nERROR: invalid result expected:0x%08lX found:0x%08lX\n", _dst, res);
        return false;                   
    }    
    // uint64_t encode = calculate_encode((uint64_t)p_dst, (uint64_t)p_src, count);
    size_t index = 0;
    size_t pre_count = (dst_offset > 0 && count > 0) ? 8 - dst_offset : 0;
    if (pre_count > count) {
        pre_count = count;
    }
    if (pre_count > 0) {
        if (!check_mop(index, encode_aligned_read((uint64_t)dst), "PRE pre write")) {
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
    memcpy((uint8_t *)prev_dst + dst_offset, fcall_ctx + FCALL_RESULT, count);
    if (memcmp(prev_dst, dst, data_size) != 0) {
        printf("\nERROR: inputcpy operation\n");
        int errors = 0;
        uint8_t *_dst = (uint8_t *)prev_dst;
        for (size_t i = 0; i < data_size; ++i) {
            if (_dst[i] == dst[i]) continue;
            printf("[%ld] 0x%02X 0x%02X NO MATCH\n", i, _dst[i], dst[i]);
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

void test_dma_inputcpy_mops() {
    printf("\x1B[1;34mTEST DMA INPUTCPY MOPS =================================================\x1B[0m\n");
    TestDmaInputCpyMops test(1024);
    test.run();
}
