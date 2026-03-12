#include <stdio.h>
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

