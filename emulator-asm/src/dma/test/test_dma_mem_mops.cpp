#include <stdio.h>
#include <algorithm>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <stdlib.h>
#include <cstdio>
#include <thread>
#include <chrono>
#include <assert.h>
#include <iostream>
#include <stdexcept>
#include <sstream>
#include <iomanip>

#include "test_dma_mem_mops.hpp"
#include "test_dma_tools.hpp"
#include "test_dma_encode.hpp"
#include "mem_config.hpp"

TestDmaMemMops::TestDmaMemMops(size_t max_count, bool use_src):
    TestDmaMem(max_count, use_src) {
}

TestDmaMemMops::~TestDmaMemMops(void) {
}

std::string TestDmaMemMops::decode(uint64_t value) {
    uint32_t flags = value >> 32;
    uint8_t bytes = flags & 0x0F;
    uint32_t addr = value & 0xFFFF'FFFF;
    uint32_t count = flags >> MOPS_BLOCK_COUNT_SBITS;
    std::ostringstream oss;
    oss << std::setfill('0') << std::setw(8) << std::hex << std::uppercase;
    switch (bytes) {
        // byte
        case 1:
        case 2:
        case 4:
        case 8: {
            if (flags & MOPS_WRITE_FLAG) {
                oss << "READ(0x";
            } else {
                oss << "WRITE(0x";
            } 
            oss << addr << "," << std::setw(0) << std::dec << bytes << ")";
            return oss.str();
        }
        case MOPS_ALIGNED_READ: {
            oss << "ALIGNED_READ(0x" << addr << ")";
            return oss.str();
        }
        case MOPS_ALIGNED_WRITE: {
            oss << "ALIGNED_WRITE(0x" << addr << ")";
            return oss.str();
        }
        case MOPS_BLOCK_READ: {
            oss << "BLOCK_READ(0x" << addr << "," << std::setw(0) << std::dec << count << ")";
            return oss.str();
        }
        case MOPS_BLOCK_WRITE: {
            oss << "BLOCK_WRITE(0x" << addr << "," << std::setw(0) << std::dec << count << ")";
            return oss.str();
        }
        case MOPS_ALIGNED_BLOCK_READ: {
            oss << "ALIGNED_BLOCK_READ(0x" << addr << "," << std::setw(0) << std::dec << count << ")";
            return oss.str();
        }
        case MOPS_ALIGNED_BLOCK_WRITE: {
            oss << "ALIGNED_BLOCK_WRITE(0x" << addr << "," << std::setw(0) << std::dec << count << ")";
            return oss.str();
        }
        default: {
            oss << "?¿ " << std::setw(2) << bytes;
            return oss.str();
        }
    }
}

void TestDmaMemMops::dump(void) {
    printf("---------------------------------\n");
    size_t trace_count = test_trace[0];
    for (size_t index = 0; index < trace_count; ++index) {
        uint64_t trace = test_trace[index+1];
        uint32_t addr = trace & 0xFFFF'FFFF;
        uint32_t flags = trace >> 32;
        printf("mops[%ld] 0x%08X_%08X %s", index, flags, addr, decode(test_trace[index+1]).c_str());
        if (src) {
            if (addr >= (uint64_t)src && addr < (uint64_t)(src + max_count)) {
                printf(" SRC+%ld", (uint64_t) addr - (uint64_t) src);
            }
        }
        if (addr >= (uint64_t)dst && addr < (uint64_t)(dst + max_count)) {
            printf(" DST+%ld", (uint64_t) addr - (uint64_t) dst);
        }
        printf("\n");
    }
}

uint64_t TestDmaMemMops::encode_read(uint32_t addr, uint8_t bytes) {    
    switch (bytes) {
        case 1:
            return (1ull << 32) | (uint64_t)addr;
        case 2:
            return (2ull << 32) | (uint64_t)addr;
        case 4:
            return (4ull << 32) | (uint64_t)addr;
        case 8:
            return (8ull << 32) | (uint64_t)addr;
        default:
            throw std::runtime_error("encode_read: invalid bytes: " + std::to_string((int)bytes));
    }
}
uint64_t TestDmaMemMops::encode_write(uint32_t addr, uint8_t bytes) {
    switch (bytes) {
        case 1:
            return ((1ull + MOPS_WRITE_FLAG) << 32) | (uint64_t)addr;
        case 2:
            return ((2ull + MOPS_WRITE_FLAG) << 32) | (uint64_t)addr;
        case 4:
            return ((4ull + MOPS_WRITE_FLAG) << 32) | (uint64_t)addr;
        case 8:
            return ((8ull + MOPS_WRITE_FLAG) << 32) | (uint64_t)addr;
        default: 
            throw std::runtime_error("encode_write: invalid bytes: " + std::to_string((int)bytes));
    }
}
uint64_t TestDmaMemMops::encode_aligned_read(uint32_t addr) {
    return ((uint64_t) MOPS_ALIGNED_READ << 32) | (uint64_t) addr;
}
uint64_t TestDmaMemMops::encode_aligned_x_read(uint32_t addr, uint32_t count) {
    if (count == 1) {
        return ((uint64_t) MOPS_ALIGNED_READ << 32) | (uint64_t) addr;
    }
    return encode_aligned_block_read(addr, count);
}
uint64_t TestDmaMemMops::encode_aligned_write(uint32_t addr) {
    return ((uint64_t) MOPS_ALIGNED_WRITE << 32) | (uint64_t) addr;
}
uint64_t TestDmaMemMops::encode_block_read(uint32_t addr, uint32_t count) {
    return ((uint64_t) MOPS_BLOCK_READ << 32) | ((uint64_t) count << (MOPS_BLOCK_COUNT_SBITS + 32)) | addr;
}
uint64_t TestDmaMemMops::encode_block_write(uint32_t addr, uint32_t count) {
    return ((uint64_t) MOPS_BLOCK_WRITE << 32) | ((uint64_t) count << (MOPS_BLOCK_COUNT_SBITS + 32)) | addr;
}
uint64_t TestDmaMemMops::encode_aligned_block_read(uint32_t addr, uint32_t count) {
    return ((uint64_t) MOPS_ALIGNED_BLOCK_READ << 32) | ((uint64_t) count << (MOPS_BLOCK_COUNT_SBITS + 32)) | addr;
}
uint64_t TestDmaMemMops::encode_aligned_block_write(uint32_t addr, uint32_t count) {
    return ((uint64_t) MOPS_ALIGNED_BLOCK_WRITE << 32) | ((uint64_t) count << (MOPS_BLOCK_COUNT_SBITS + 32)) | addr;

}


bool TestDmaMemMops::check_record(size_t &w, uint64_t expected, const char *tag) {
    const uint64_t mode = (expected >> 32) & 0x0F;
    const bool block = (mode == MOPS_BLOCK_READ) || (mode == MOPS_BLOCK_WRITE) ||
                       (mode == MOPS_ALIGNED_BLOCK_READ) || (mode == MOPS_ALIGNED_BLOCK_WRITE);
#ifdef MOPS_LIGHT
    // Light form: one word per record, no step; a block record carries the no-payload bit.
    const uint64_t header = expected | TAG | (block ? (1ull << MOPS_NO_PAYLOAD_BIT) : 0);
    const bool ok = mtrace[w] == header;
    const size_t len = 1;
#else
    const uint64_t header = (block ? expected : (expected | step_field(2))) | TAG;
    const bool ok = mtrace[w] == header && (!block || mtrace[w + 1] == step_field(2));
    const size_t len = block ? 2 : 1;
#endif
    if (!ok) {
        printf("\nERROR: %s expected: 0x%016lX (%s) found: mtrace[%ld]:0x%016lX/0x%016lX (%s)\n", tag,
               header, decode(expected).c_str(), w, mtrace[w], mtrace[w + 1], decode(mtrace[w] & ~TAG).c_str());
        return false;
    }
    w += len;
    return true;
}

bool TestDmaMemMops::check_write_records(size_t &w, uint64_t addr, size_t words, const char *tag) {
#ifdef MOPS_LIGHT
    // Light form: the block-write descriptor is the record, one word, no values.
    const uint64_t header = addr | ((uint64_t)MOPS_ALIGNED_BLOCK_WRITE << 32) | ((uint64_t)words << 36)
                            | (1ull << MOPS_NO_PAYLOAD_BIT) | TAG;
    if (mtrace[w] != header) {
        printf("\nERROR: %s block write at word %ld expected header 0x%016lX found 0x%016lX\n", tag, w, header, mtrace[w]);
        return false;
    }
    w += 1;
    return true;
#endif
    const uint64_t wstep = step_field(3) << 4;   // the value block keeps the step field at bit 42
    size_t k = 0;
    while (k < words) {
        const size_t n = std::min<size_t>(63, words - k);
        const uint64_t header = (addr + k * 8) | ((uint64_t)MOPS_BLOCK_VALUES << 32) | ((uint64_t)n << 36) | wstep | TAG;
        if (mtrace[w] != header) {
            printf("\nERROR: %s block at word %ld expected header 0x%016lX found 0x%016lX\n", tag, w, header, mtrace[w]);
            return false;
        }
        uint64_t tops = 0;
        for (size_t i = 0; i < n; ++i) {
            uint64_t value;
            memcpy(&value, (const void *)(addr + (k + i) * 8), sizeof(value));
            tops |= (value >> 63) << i;
            if (mtrace[w + 2 + i] != (value & ~TAG)) {
                printf("\nERROR: %s word %ld expected value 0x%016lX found 0x%016lX\n", tag, k + i, value & ~TAG, mtrace[w + 2 + i]);
                return false;
            }
        }
        if (mtrace[w + 1] != tops) {
            printf("\nERROR: %s block at word %ld expected top bits 0x%016lX found 0x%016lX\n", tag, w, tops, mtrace[w + 1]);
            return false;
        }
        w += 2 + n;
        k += n;
    }
    return true;
}

