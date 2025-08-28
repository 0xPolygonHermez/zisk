#ifndef __MEM_COUNTER_HPP__
#define __MEM_COUNTER_HPP__

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

#include "mem_config.hpp"
#include "mem_types.hpp"
#include "mem_context.hpp"
#include "tools.hpp"
#include "mem_stats.hpp"

#ifdef USE_ADDR_COUNT_TABLE
struct AddrCount {
    uint32_t pos;
    uint32_t count;
};
#define ADDR_TABLE_ELEMENT_SIZE sizeof(AddrCount)
#else
#define ADDR_TABLE_ELEMENT_SIZE sizeof(uint32_t)
#endif
class MemCounter {
private:
    const uint32_t id;
    std::shared_ptr<MemContext> context;
    int count;
    int addr_count;

    #ifdef USE_ADDR_COUNT_TABLE
    AddrCount *addr_count_table;
    #else
    uint32_t *addr_table;
    #endif
    uint32_t *addr_slots;
    uint32_t current_chunk;
    uint32_t free_slot;
    uint32_t elapsed_ms;
    uint32_t queue_full;
    uint64_t first_chunk_us;
    const uint32_t addr_mask;
#ifdef COUNT_CHUNK_STATS
    uint64_t chunks_us[MAX_CHUNKS];
    int64_t wait_chunks_us[MAX_CHUNKS];
#endif

#ifdef MEM_STATS_ACTIVE
public:
    MemStats *mem_stats;
#endif // MEM_STATS_ACTIVE

public:
    MemCounter(const MemCounter&) = delete;
    MemCounter& operator=(const MemCounter&) = delete;
    MemCounter(MemCounter&&) noexcept = delete;

    uint32_t first_offset[MAX_PAGES];
    uint32_t last_offset[MAX_PAGES];
    uint64_t tot_wait_us;
    MemCounter(uint32_t id, std::shared_ptr<MemContext> context);
    inline uint32_t get_count();
    inline uint32_t get_used_slots();
    ~MemCounter();
    void execute();
    void execute_chunk(uint32_t chunk_id, const MemCountersBusData *chunk_data, uint32_t chunk_size);
    inline uint32_t get_initial_block_pos(uint32_t pos);
    inline uint32_t get_final_block_pos(uint32_t pos);
    inline uint32_t get_next_block(uint32_t pos);
    inline uint32_t get_initial_pos(uint32_t pos) const;
    inline uint32_t get_pos_value(uint32_t pos) const;
    inline uint32_t get_queue_full_times() const;
    inline uint32_t get_next_pos(uint32_t pos) const;
    inline uint32_t get_addr_table(uint32_t index) const;
    inline uint32_t get_count_table(uint32_t index) const;
    inline uint32_t get_next_slot_pos();
    void count_aligned(uint32_t addr, uint32_t chunk_id, uint32_t count);
    
    uint32_t get_elapsed_ms() {
        return elapsed_ms;
    }
    inline static uint32_t offset_to_page(uint32_t offset);
    inline static void offset_info(uint32_t offset, uint32_t &page, uint32_t &addr, uint32_t thread_index);        
    inline static uint32_t offset_to_addr(uint32_t offset, uint32_t thread_index);
    inline static uint32_t addr_to_offset(uint32_t addr, uint32_t chunk_id = 0);
    inline static uint32_t addr_to_page(uint32_t addr, uint32_t chunk_id = 0);
    inline static uint32_t page_to_addr(uint8_t page);
    inline uint32_t get_used_slots(void) const;
    inline uint64_t get_first_chunk_us(void) const;
    void stats();
};

uint32_t MemCounter::get_pos_value(uint32_t pos) const {
    return addr_slots[pos];
}

uint32_t MemCounter::get_count() {
    return addr_count;
}

uint32_t MemCounter::get_used_slots() {
    return free_slot;
}

uint32_t MemCounter::get_initial_block_pos(uint32_t pos) {
    uint32_t tpos = pos & ADDR_SLOT_MASK;
    if (addr_slots[tpos] == 0) {
        return tpos;
    } else {
        return addr_slots[tpos+1];
    }
}

uint32_t MemCounter::get_final_block_pos(uint32_t pos) {
    return pos & ADDR_SLOT_MASK;
}

uint32_t MemCounter::get_next_block(uint32_t pos) {
    return addr_slots[pos+1];
}

uint32_t MemCounter::get_initial_pos(uint32_t pos) const {
    uint32_t tpos = pos & ADDR_SLOT_MASK;
    if (tpos >= ADDR_SLOTS_SIZE) {
        std::ostringstream msg;
        msg << "Error: get_initial_pos: " << tpos << " out of bounds " << ADDR_SLOTS_SIZE << " (pos:" << pos << ")\n";
        throw std::runtime_error(msg.str());
    }
    if (addr_slots[tpos] == 0) {
        return tpos + 2;
    } else {
        return addr_slots[tpos+1] + 2;
    }
}

uint32_t MemCounter::get_queue_full_times() const {
    return queue_full;
}

uint64_t MemCounter::get_first_chunk_us() const {
    return first_chunk_us;
}

uint32_t MemCounter::get_next_pos(uint32_t pos) const {
    int relative_pos = pos & (ADDR_SLOT_SIZE - 1);
    if (relative_pos < (ADDR_SLOT_SIZE - 1)) {
        return pos + 1;
    }
    uint32_t tpos = pos & ADDR_SLOT_MASK;
    if (addr_slots[tpos+1] != 0) {
        return addr_slots[tpos+1]+2;
    }
    return 0;
}

uint32_t MemCounter::get_addr_table(uint32_t index) const {
    #ifdef USE_ADDR_COUNT_TABLE
    return addr_count_table[index].pos;
    #else
    return addr_table[index];
    #endif
}

uint32_t MemCounter::get_count_table(uint32_t index) const {
    // return count_table[index];
    #ifdef USE_ADDR_COUNT_TABLE
    return addr_count_table[index].count;
    #else
    return addr_table[index];
    #endif
}

uint32_t MemCounter::get_next_slot_pos() {
    if (free_slot >= ADDR_SLOTS) {
        std::ostringstream msg;
        msg << "ERROR: MemCounter no more free slots on thread" << id;
        throw std::runtime_error(msg.str());
    }
    return (free_slot++) * ADDR_SLOT_SIZE;
}

uint32_t MemCounter::offset_to_page(uint32_t offset) {
    return (offset >> ADDR_PAGE_BITS);
}

void MemCounter::offset_info(uint32_t offset, uint32_t &page, uint32_t &addr, uint32_t thread_index) {
    page = offset >> ADDR_PAGE_BITS;
    uint32_t base_addr = page_to_addr(page);
    addr = ((offset & RELATIVE_OFFSET_MASK) << ADDR_LOW_BITS) + base_addr + thread_index * 8;
}

uint32_t MemCounter::offset_to_addr(uint32_t offset, uint32_t thread_index) {
    uint32_t page = offset >> ADDR_PAGE_BITS;
    uint32_t base_addr = page_to_addr(page);
    return ((offset & RELATIVE_OFFSET_MASK) << ADDR_LOW_BITS) + base_addr + thread_index * 8;
}

#define RAM_ADDR_MASK (RAM_ADDR >> 24)
#define ROM_ADDR_MASK (ROM_ADDR >> 24)
#define INPUT_ADDR_MASK (INPUT_ADDR >> 24)

uint32_t MemCounter::addr_to_offset(uint32_t addr, uint32_t chunk_id) {
    switch((uint8_t)((addr >> 24) & 0xFC)) {
        case (ROM_ADDR_MASK + 0x00): return ((addr - (ROM_ADDR + 0x00000000)) >> (ADDR_LOW_BITS));
        case (ROM_ADDR_MASK + 0x04): return ((addr - (ROM_ADDR + 0x04000000)) >> (ADDR_LOW_BITS)) + ADDR_PAGE_SIZE;
        case (INPUT_ADDR_MASK + 0x00): return ((addr - (INPUT_ADDR + 0x00000000)) >> (ADDR_LOW_BITS)) + 2 * ADDR_PAGE_SIZE;
        case (INPUT_ADDR_MASK + 0x04): return ((addr - (INPUT_ADDR + 0x04000000)) >> (ADDR_LOW_BITS)) + 3 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x00): return ((addr - (RAM_ADDR + 0x00000000)) >> (ADDR_LOW_BITS)) + 4 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x04): return ((addr - (RAM_ADDR + 0x04000000)) >> (ADDR_LOW_BITS)) + 5 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x08): return ((addr - (RAM_ADDR + 0x08000000)) >> (ADDR_LOW_BITS)) + 6 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x0C): return ((addr - (RAM_ADDR + 0x0C000000)) >> (ADDR_LOW_BITS)) + 7 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x10): return ((addr - (RAM_ADDR + 0x10000000)) >> (ADDR_LOW_BITS)) + 8 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x14): return ((addr - (RAM_ADDR + 0x14000000)) >> (ADDR_LOW_BITS)) + 9 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x18): return ((addr - (RAM_ADDR + 0x18000000)) >> (ADDR_LOW_BITS)) + 10 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x1C): return ((addr - (RAM_ADDR + 0x1C000000)) >> (ADDR_LOW_BITS)) + 11 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x20): return ((addr - (RAM_ADDR + 0x20000000)) >> (ADDR_LOW_BITS)) + 12 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x24): return ((addr - (RAM_ADDR + 0x24000000)) >> (ADDR_LOW_BITS)) + 13 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x28): return ((addr - (RAM_ADDR + 0x28000000)) >> (ADDR_LOW_BITS)) + 14 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x2C): return ((addr - (RAM_ADDR + 0x2C000000)) >> (ADDR_LOW_BITS)) + 15 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x30): return ((addr - (RAM_ADDR + 0x30000000)) >> (ADDR_LOW_BITS)) + 16 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x34): return ((addr - (RAM_ADDR + 0x34000000)) >> (ADDR_LOW_BITS)) + 17 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x38): return ((addr - (RAM_ADDR + 0x38000000)) >> (ADDR_LOW_BITS)) + 18 * ADDR_PAGE_SIZE;
        case (RAM_ADDR_MASK + 0x3C): return ((addr - (RAM_ADDR + 0x3C000000)) >> (ADDR_LOW_BITS)) + 19 * ADDR_PAGE_SIZE;
    }
    std::ostringstream msg;
    msg << "ERROR: addr_to_offset: 0x" << std::hex << addr << " (" << std::dec << chunk_id << ")";
    throw std::runtime_error(msg.str());
}

uint32_t MemCounter::addr_to_page(uint32_t addr, uint32_t chunk_id) {
    switch((uint8_t)((addr >> 24) & 0xFC)) {
        case (ROM_ADDR_MASK + 0x00): return 0;
        case (ROM_ADDR_MASK + 0x04): return 1;
        case (INPUT_ADDR_MASK + 0x00): return 2;
        case (INPUT_ADDR_MASK + 0x04): return 3;
        case (RAM_ADDR_MASK + 0x00): return 4;
        case (RAM_ADDR_MASK + 0x04): return 5;
        case (RAM_ADDR_MASK + 0x08): return 6;
        case (RAM_ADDR_MASK + 0x0C): return 7;
        case (RAM_ADDR_MASK + 0x10): return 8;
        case (RAM_ADDR_MASK + 0x14): return 9;
        case (RAM_ADDR_MASK + 0x18): return 10;
        case (RAM_ADDR_MASK + 0x1C): return 11;
        case (RAM_ADDR_MASK + 0x20): return 12;
        case (RAM_ADDR_MASK + 0x24): return 13;
        case (RAM_ADDR_MASK + 0x28): return 14;
        case (RAM_ADDR_MASK + 0x2C): return 15;
        case (RAM_ADDR_MASK + 0x30): return 16;
        case (RAM_ADDR_MASK + 0x34): return 17;
        case (RAM_ADDR_MASK + 0x38): return 18;
        case (RAM_ADDR_MASK + 0x3C): return 19;
    }
    std::ostringstream msg;
    msg << "ERROR: addr_to_page: 0x" << std::hex << addr << " (" << std::dec << chunk_id << ")";
    throw std::runtime_error(msg.str());
}

uint32_t MemCounter::page_to_addr(uint8_t page) {
    switch(page) {
        case 0: return (ROM_ADDR + 0x00000000);
        case 1: return (ROM_ADDR + 0x04000000);
        case 2: return (INPUT_ADDR + 0x00000000);
        case 3: return (INPUT_ADDR + 0x04000000);
        case 4: return (RAM_ADDR + 0x00000000);
        case 5: return (RAM_ADDR + 0x04000000);
        case 6: return (RAM_ADDR + 0x08000000);
        case 7: return (RAM_ADDR + 0x0C000000);
        case 8: return (RAM_ADDR + 0x10000000);
        case 9: return (RAM_ADDR + 0x14000000);
        case 10: return (RAM_ADDR + 0x18000000);
        case 11: return (RAM_ADDR + 0x1C000000);
        case 12: return (RAM_ADDR + 0x20000000);
        case 13: return (RAM_ADDR + 0x24000000);
        case 14: return (RAM_ADDR + 0x28000000);
        case 15: return (RAM_ADDR + 0x2C000000);
        case 16: return (RAM_ADDR + 0x30000000);
        case 17: return (RAM_ADDR + 0x34000000);
        case 18: return (RAM_ADDR + 0x38000000);
        case 19: return (RAM_ADDR + 0x3C000000);
        case 0xFF: return 0xFFFFFFFF;
    }
    std::ostringstream msg;
    msg << "ERROR: MemCounter page_to_address page:" << page;
    throw std::runtime_error(msg.str());
}

uint32_t MemCounter::get_used_slots(void) const {
        return free_slot;
}
#endif
