#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <cstdio>
#include <thread>
#include <chrono>
#include <assert.h>
#include "test_dma_mem_mtrace.hpp"
#include "test_dma_inputcpy_mtrace.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"
#include "test_mock.hpp"

extern "C" {
    uint64_t test_asm_dma_inputcpy_mtrace(uint8_t *dst, uint8_t *src, size_t count, uint64_t *trace);
}

class TestDmaInputCpyMtrace: public TestDmaMemMtrace {
protected:
    uint64_t *prev_dst;
    uint64_t *check_dst;
    bool execute_single_test(void);
public:
    TestDmaInputCpyMtrace(size_t max_count = 1024);
    virtual ~TestDmaInputCpyMtrace();
    void run(void);
};

TestDmaInputCpyMtrace::TestDmaInputCpyMtrace(size_t max_count):
    TestDmaMemMtrace(max_count, false) {
    prev_dst = (uint64_t *)malloc(data_size);
    check_dst = (uint64_t *)malloc(data_size);
}

TestDmaInputCpyMtrace::~TestDmaInputCpyMtrace(void) {
    free(prev_dst);
    free(check_dst);
}

void TestDmaInputCpyMtrace::run(void) {
    fill_pattern((uint8_t *)(fcall_ctx + FCALL_RESULT), FCALL_RESULT_LENGTH, 3013102105130209);
    size_t total_tests = 0;
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

bool TestDmaInputCpyMtrace::execute_single_test(void) {
    fill_pattern((uint8_t *)(fcall_ctx + FCALL_RESULT), FCALL_RESULT_LENGTH, 15436 + dst_offset + count);
    fcall_ctx[FCALL_RESULT_GOT] = 1;
    fill_pattern(dst, data_size, 1821904675);
    uint8_t *p_dst = dst + dst_offset;

    memcpy(prev_dst, dst, data_size);
    printf("\rTEST dst_offset:%ld count:%4ld", dst_offset, count);
    fflush(stdout);
    uint64_t res = test_asm_dma_inputcpy_mtrace(p_dst, 0, count, test_trace);
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
    uint64_t encode = calculate_encode_inputcpy((uint64_t)p_dst, count);
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
    size_t input_qwords = count > 0 ? (count + 7) >> 3 : 0;
    for (size_t i = 0; i < input_qwords; ++i) {
        uint64_t expected = fcall_ctx[FCALL_RESULT + i];
        if (mtrace[index] != expected) {
            printf("\nERROR: input value expected: input[%ld]:0x%016lX vs found mtrace[%ld]:0x%016lX\n", i, expected, index, mtrace[index]);
            return false;
        }
        ++index;
    }
    if (trace_count != index) {
        printf("\nERROR: invalid mtrace len expected:%ld vs found:%ld\n", index, trace_count);
        size_t _count = index > trace_count ? index + 1: trace_count + 1;
        for (size_t i = 0; i <= _count; ++i) {
            printf("mtrace[%ld] 0x%016lX\n", i, mtrace[i]);
        }
        return false;
    }
    memcpy((uint8_t *)prev_dst + dst_offset, fcall_ctx + FCALL_RESULT, count);
    if (memcmp(prev_dst, dst, data_size) != 0) {
        printf("\nERROR: inputcpy operation\n");

        return false;
    }
    return true;
}

void test_dma_inputcpy_mtrace() {
    printf("\x1B[1;34mTEST DMA INPUTCPY MTRACE =================================================\x1B[0m\n");
    TestDmaInputCpyMtrace test(1024);
    test.run();
}
