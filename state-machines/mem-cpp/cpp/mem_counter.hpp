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

#define ST_BITS_OFFSET 30
#define ST_BITS_ST_MASK (0xFFFFFFFF << ST_BITS_OFFSET)
#define ST_BITS_COUNTER_MASK (0xFFFFFFFF >> (32 - ST_BITS_OFFSET))

struct AddrCount {
    uint32_t pos;
    uint32_t count;
};
#define ADDR_TABLE_ELEMENT_SIZE sizeof(AddrCount)
class MemCounter {
private:
    const uint32_t id;
    std::shared_ptr<MemContext> context;
    int count;
    int addr_count;

    AddrCount *addr_count_table;
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
    inline uint32_t get_pos_count(uint32_t pos) const;
    inline uint32_t get_queue_full_times() const;
    inline uint32_t get_next_pos(uint32_t pos) const;
    inline uint32_t get_addr_table(uint32_t index) const;
    inline uint32_t get_count_table(uint32_t index) const;
    inline uint32_t get_next_slot_pos();
    inline void update_addr_count(uint32_t &count, bool is_aligned, bool is_write, bool is_ram);
    inline uint32_t init_addr_count(bool is_aligned, bool is_write, bool is_ram);
    void incr_counter(uint32_t addr, uint32_t chunk_id, bool is_aligned, bool is_write);
    inline uint32_t incr_st_counter_aligned(uint32_t count, bool is_write);
    inline uint32_t incr_st_counter_unaligned(uint32_t count, bool is_write);

    
    uint32_t get_elapsed_ms() {
        return elapsed_ms;
    }
    inline static uint32_t offset_to_page(uint32_t offset);
    inline static void offset_info(uint32_t offset, uint32_t &page, uint32_t &addr, uint32_t thread_index);        
    inline static uint32_t offset_to_addr(uint32_t offset, uint32_t thread_index);
    inline static uint32_t addr_to_offset(uint32_t addr, uint32_t chunk_id = 0);
    inline static uint32_t addr_to_page(uint32_t addr, uint32_t chunk_id = 0);
    inline static uint32_t page_to_addr(uint8_t page);
    inline static const char *page_to_tag(uint8_t page);
    inline uint32_t get_used_slots(void) const;
    inline uint64_t get_first_chunk_us(void) const;
    void stats();
};

uint32_t MemCounter::get_pos_value(uint32_t pos) const {
    return addr_slots[pos];
}

uint32_t MemCounter::get_pos_count(uint32_t pos) const {    
    return (addr_slots[pos] & ST_BITS_COUNTER_MASK) + ((addr_slots[pos] & ST_BITS_ST_MASK) ? 1:0);
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
    if (addr_count_table[index].pos == 0) {
        return 0;
    }    
    return addr_count_table[index].count + get_pos_count(addr_count_table[index].pos + 1);
}

uint32_t MemCounter::get_next_slot_pos() {
    if (free_slot >= ADDR_SLOTS) {
        std::ostringstream msg;
        msg << "ERROR: MemCounter: no free slots left for this thread(" << id << "). Increase MAX_SLOT_GB in state-machines/mem-cpp/cpp/mem_config.hpp and recompile zisk.";
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

#define ROM_FIRST_BLOCK (ROM_ADDR >> ADDR_BLOCK_BITS)
#define INPUT_FIRST_BLOCK (INPUT_ADDR >> ADDR_BLOCK_BITS)
#define RAM_FIRST_BLOCK (RAM_ADDR >> ADDR_BLOCK_BITS)
#define ADDR_BLOCK_MASK ((1 << ADDR_BLOCK_BITS) - 1)

uint32_t MemCounter::addr_to_offset(uint32_t addr, uint32_t chunk_id) {
    return (addr_to_page(addr, chunk_id) << ADDR_PAGE_BITS) + ((addr & ADDR_BLOCK_MASK) >> ADDR_LOW_BITS);
}
const char *MemCounter::page_to_tag(uint8_t page) {
    if (page < INPUT_FIRST_PAGE) return "rom_data";
    if (page < RAM_FIRST_PAGE) return "input_data";
    if (page < MAX_PAGES) return "mem";
    return "??";
}
uint32_t MemCounter::addr_to_page(uint32_t addr, uint32_t chunk_id) {
    // unsigned subtraction: a block below the first block of the area wraps to a large value
    const uint32_t block = addr >> ADDR_BLOCK_BITS;
    if ((block - RAM_FIRST_BLOCK) < RAM_PAGES) return RAM_FIRST_PAGE + (block - RAM_FIRST_BLOCK);
    if ((block - INPUT_FIRST_BLOCK) < INPUT_PAGES) return INPUT_FIRST_PAGE + (block - INPUT_FIRST_BLOCK);
    if ((block - ROM_FIRST_BLOCK) < ROM_PAGES) return ROM_FIRST_PAGE + (block - ROM_FIRST_BLOCK);
    std::ostringstream msg;
    msg << "ERROR: addr_to_page: 0x" << std::hex << addr << " (" << std::dec << chunk_id << ")";
    throw std::runtime_error(msg.str());
}

uint32_t MemCounter::page_to_addr(uint8_t page) {
    if (page < INPUT_FIRST_PAGE) return ROM_ADDR + ((uint32_t)(page - ROM_FIRST_PAGE) << ADDR_BLOCK_BITS);
    if (page < RAM_FIRST_PAGE) return INPUT_ADDR + ((uint32_t)(page - INPUT_FIRST_PAGE) << ADDR_BLOCK_BITS);
    if (page < MAX_PAGES) return RAM_ADDR + ((uint32_t)(page - RAM_FIRST_PAGE) << ADDR_BLOCK_BITS);
    if (page == 0xFF) return 0xFFFFFFFF;
    std::ostringstream msg;
    msg << "ERROR: MemCounter page_to_address page: " << (int)page;
    throw std::runtime_error(msg.str());
}

uint32_t MemCounter::get_used_slots(void) const {
        return free_slot;
}
#endif
