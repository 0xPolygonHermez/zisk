#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <cstdio>
#include <thread>
#include <chrono>
#include <assert.h>
#include "test_dma_mem_mtrace.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"

TestDmaMemMtrace::TestDmaMemMtrace(size_t max_count, bool use_src):
    TestDmaMem(max_count, use_src) {
}

TestDmaMemMtrace::~TestDmaMemMtrace(void) {
}

void TestDmaMemMtrace::dump(void) {
    printf("---------------------------------\n");
    size_t dst_qwords = (dst_offset + count + 7) >> 3;
    for (size_t index = 0; index < dst_qwords; ++index) {
        printf("dst64[%ld] 0x%016lX\n", index, aligned_dst[index]);
    }
    if (src) {
        printf("---------------------------------\n");
        size_t src_qwords = (src_offset + count + 7) >> 3;
        for (size_t index = 0; index < src_qwords; ++index) {
            printf("src64[%ld] 0x%016lX\n", index, aligned_src[index]);
        }
    }
    printf("---------------------------------\n");
    size_t trace_count = test_trace[0];
    for (size_t index = 0; index < trace_count; ++index) {
        printf("mtrace[%ld] 0x%016lX\n", index, test_trace[index+1]);
    }
}
