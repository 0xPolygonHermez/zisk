#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <cstdio>
#include <thread>
#include <chrono>
#include <assert.h>
#include "test_dma_mem.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"

TestDmaMem::TestDmaMem(size_t max_count, bool use_src):
    max_count(max_count), use_src(use_src) {
    data_size = sizeof(uint64_t) * (max_count + 16);
    trace_size = sizeof(uint64_t) * max_count * 4;
    src = use_src ? (uint8_t *)malloc(data_size) : NULL;
    dst = (uint8_t *)malloc(data_size);
    aligned_dst = (uint64_t *)dst;
    aligned_src = (uint64_t *)src;
    test_trace = (uint64_t *)malloc(trace_size);
    mtrace = test_trace + 1;
}

TestDmaMem::~TestDmaMem(void) {
    if (src) free(src);
    free(dst);
    free(test_trace);
}