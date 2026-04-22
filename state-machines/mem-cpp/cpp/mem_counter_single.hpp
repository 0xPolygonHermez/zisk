#ifndef __MEM_COUNTER_SINGLE_HPP__
#define __MEM_COUNTER_SINGLE_HPP__

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>
#include <vector>
#include <thread>
#include <iostream>
#include <string.h>
#include <sys/time.h>
#include <cstdint>
#include <vector>
#include <stdexcept>
#include <sstream>
#include <memory>

#include "mem_types.hpp"

#define RAM_SIZE (RAM_SIZE_MB * 1024 * 1024)
#define ROM_SIZE (ROM_SIZE_MB * 1024 * 1024)
#define INPUT_SIZE (INPUT_SIZE_MB * 1024 * 1024)
#define TABLE_OFFSET_SIZE ((RAM_SIZE + ROM_SIZE + INPUT_SIZE) >> 3)

class MemCounterSingle {
    bool *dual_available;
    uint32_t *counter;
    uint32_t full_5;
    uint32_t full_3;
    uint32_t full_2;
    uint32_t read_byte;
    uint32_t write_byte;

public:
    MemCounterSingle(void);
    ~MemCounterSingle();
    void execute(const MemCountersBusData *chunk_data, uint32_t chunk_size);
    void count_aligned(uint32_t addr, bool is_write);
    void count_aligned_write(uint32_t addr) { count_aligned(addr, true); }
    void count_aligned_read(uint32_t addr) { count_aligned(addr, false); }
    
    inline static uint32_t addr_to_offset(uint32_t addr);
};

uint32_t MemCounterSingle::addr_to_offset(uint32_t addr) {
    if (addr >= RAM_ADDR && addr < (RAM_ADDR + RAM_SIZE)) {
        return (addr - RAM_ADDR) >> 3;
    }
    if (addr >= ROM_ADDR && addr < (ROM_ADDR + ROM_SIZE)) {
        return ((addr - ROM_ADDR) >> 3) + (RAM_SIZE >> 3);
    }
    if (addr >= INPUT_ADDR && addr < (INPUT_ADDR + INPUT_SIZE)) {
        return ((addr - INPUT_ADDR) >> 3) + (RAM_SIZE >> 3) + (ROM_SIZE >> 3);
    }
    std::ostringstream msg;
    msg << "ERROR: addr_to_offset: 0x" << std::hex << addr;
    throw std::runtime_error(msg.str());
}

#endif
