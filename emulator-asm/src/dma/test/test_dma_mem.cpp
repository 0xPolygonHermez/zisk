#include <stdio.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <cstdio>
#include <thread>
#include <chrono>
#include <assert.h>
#include "test_dma_mem.hpp"
#include <sys/mman.h>
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
    if (low_mapped) {
        if (src) munmap(src, data_size);
        munmap(dst, data_size);
        munmap(test_trace, trace_size);
        return;
    }
    if (src) free(src);
    free(dst);
    free(test_trace);
}

void TestDmaMem::map_low(uint64_t base) {
    auto map = [&](size_t bytes) -> uint8_t * {
        void *p = mmap((void *)base, bytes, PROT_READ | PROT_WRITE,
                       MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED_NOREPLACE, -1, 0);
        if (p == MAP_FAILED || (uint64_t)p != base) {
            printf("ERROR: cannot map %ld bytes at 0x%lX\n", bytes, base);
            exit(1);
        }
        base += (bytes + 0xFFFFF) & ~(uint64_t)0xFFFFF;
        return (uint8_t *)p;
    };
    if (src) { free(src); src = map(data_size); }
    free(dst);
    dst = map(data_size);
    free(test_trace);
    test_trace = (uint64_t *)map(trace_size);
    aligned_dst = (uint64_t *)dst;
    aligned_src = (uint64_t *)src;
    mtrace = test_trace + 1;
    low_mapped = true;
}