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

#include "mem_config.hpp"
#include "mem_types.hpp"
#include "mem_context.hpp"
#include "tools.hpp"

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
    MemContext *context;
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
    uint32_t tot_usleep;
    uint32_t queue_full;
    const uint32_t addr_mask;
public:
    uint32_t first_offset[MAX_PAGES];
    uint32_t last_offset[MAX_PAGES];
    MemCounter(uint32_t id, MemContext *context)
    :id(id), context(context), addr_mask(id * 8) {
        count = 0;
        queue_full = 0;
        tot_usleep = 0;
        #ifdef USE_ADDR_COUNT_TABLE
        addr_count_table = (AddrCount *)malloc(ADDR_TABLE_SIZE * sizeof(AddrCount));
        memset(addr_count_table, 0, ADDR_TABLE_SIZE * sizeof(AddrCount));
        #else
        addr_table = (uint32_t *)malloc(ADDR_TABLE_SIZE * sizeof(uint32_t));
        memset(addr_table, 0, ADDR_TABLE_SIZE * sizeof(uint32_t));
        #endif


        // no memset because informations is overrided.
        addr_slots = (uint32_t *)std::aligned_alloc(64, ADDR_SLOTS_SIZE * sizeof(uint32_t));

        memset(first_offset, 0xFF, sizeof(first_offset));
        memset(last_offset, 0, sizeof(first_offset));

        free_slot = 0;
        addr_count = 0;
    }
    uint32_t get_count() {
        return addr_count;
    }
    uint32_t get_used_slots() {
        return free_slot;
    }
    uint32_t get_tot_usleep() {
        return tot_usleep;
    }
    ~MemCounter() {
        #ifdef USE_ADDR_COUNT_TABLE
        free(addr_count_table);
        #else
        free(addr_table);
        #endif
        free(addr_slots);
    }
    void execute() {
        uint64_t init = get_usec();
        const MemChunk *chunk;
        uint32_t chunk_id = 0;
        while ((chunk = context->get_chunk(chunk_id)) != nullptr) {
            execute_chunk(chunk_id, chunk->data, chunk->count);
            ++chunk_id;
        }
        elapsed_ms = ((get_usec() - init) / 1000);
    }
    void execute_chunk(uint32_t chunk_id, const MemCountersBusData *chunk_data, uint32_t chunk_size) {
        current_chunk = chunk_id;

        for (const MemCountersBusData *chunk_eod = chunk_data + chunk_size; chunk_eod != chunk_data; chunk_data++) {
            const uint8_t bytes = chunk_data->flags & 0xFF;
            const uint32_t addr = chunk_data->addr;
            switch (bytes) {
                case 1: // byte
                case 2: // half word
                case 4: // word
                case 8: // double word
                    break;
                default:
                    std::ostringstream msg;
                    msg << "ERROR: MemCounter execute_chunk: invalid bytes size " << bytes << " at chunk_id " << chunk_id << " addr 0x" << std::hex << addr;
                    throw std::runtime_error(msg.str());
            }
            if (bytes == 8 && (addr & 0x07) == 0) {
                // aligned access
                if ((addr & ADDR_MASK) != addr_mask) {
                    continue;
                }
                count_aligned(addr, chunk_id, 1);
            } else {
                const uint32_t aligned_addr = addr & 0xFFFFFFF8;

                if ((aligned_addr & ADDR_MASK) == addr_mask) {
                    const int ops = 1 + (chunk_data->flags >> 16);
                    count_aligned(aligned_addr, chunk_id, ops);
                }
                else if ((bytes + (addr & 0x07)) > 8 && ((aligned_addr + 8) & ADDR_MASK) == addr_mask) {
                    const int ops = 1 + (chunk_data->flags >> 16);
                    count_aligned(aligned_addr + 8 , chunk_id, ops);
                }
            }
        }
    }
    inline uint32_t get_initial_block_pos(uint32_t pos) {
        uint32_t tpos = pos & ADDR_SLOT_MASK;
        if (addr_slots[tpos] == 0) {
            return tpos;
        } else {
            return addr_slots[tpos+1];
        }
    }
    inline uint32_t get_final_block_pos(uint32_t pos) {
        return pos & ADDR_SLOT_MASK;
    }

    inline uint32_t get_next_block(uint32_t pos) {
        return addr_slots[pos+1];
    }

    inline uint32_t get_initial_pos(uint32_t pos) const {
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

    inline uint32_t get_pos_value(uint32_t pos) const {
        return addr_slots[pos];
    }
    inline uint32_t get_queue_full_times() const {
        return queue_full;
    }
    inline uint32_t get_next_pos(uint32_t pos) const {
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
    inline uint32_t get_addr_table(uint32_t index) const {
        #ifdef USE_ADDR_COUNT_TABLE
        return addr_count_table[index].pos;
        #else
        return addr_table[index];
        #endif
    }
    inline uint32_t get_count_table(uint32_t index) const {
        // return count_table[index];
        #ifdef USE_ADDR_COUNT_TABLE
        return addr_count_table[index].count;
        #else
        return addr_table[index];
        #endif
    }
    inline uint32_t get_next_slot_pos() {
        if (free_slot >= ADDR_SLOTS) {
            std::ostringstream msg;
            msg << "ERROR: MemCounter no more free slots on thread" << id;
            throw std::runtime_error(msg.str());
        }
        return (free_slot++) * ADDR_SLOT_SIZE;
    }
    inline void count_aligned(uint32_t addr, uint32_t chunk_id, uint32_t count) {
        uint32_t offset = addr_to_offset(addr, current_chunk);
        #ifdef USE_ADDR_COUNT_TABLE
        uint32_t pos = addr_count_table[offset].pos;
        #else
        uint32_t pos = addr_table[offset];
        #endif
        if (pos == 0) {
            uint32_t pos = get_next_slot_pos();
            addr_slots[pos] = 0;
            addr_slots[pos + 1] = pos;
            addr_slots[pos + 2] = chunk_id;
            addr_slots[pos + 3] = count;
            #ifdef USE_ADDR_COUNT_TABLE
            addr_count_table[offset].pos = pos + 2;
            addr_count_table[offset].count = count;
            #else
            addr_table[offset] = pos + 2;
            #endif
            uint32_t page = offset >> ADDR_PAGE_BITS;
            first_offset[page] = std::min(first_offset[page], offset);
            last_offset[page] = std::max(last_offset[page], offset);
            ++addr_count;
        } else {
            #ifdef USE_ADDR_COUNT_TABLE
            addr_count_table[offset].count += count;
            #endif
            if (addr_slots[pos] == chunk_id) {
                addr_slots[pos + 1] += count;
                return;
            }
            if ((pos % ADDR_SLOT_SIZE) == (ADDR_SLOT_SIZE - 2)) {
                uint32_t npos = get_next_slot_pos();
                uint32_t tpos = pos - ADDR_SLOT_SIZE + 2;
                addr_slots[npos] = tpos;
                addr_slots[npos + 1] = addr_slots[tpos + 1];
                addr_slots[npos + 2] = chunk_id;
                addr_slots[npos + 3] = count;
                addr_slots[tpos + 1] = npos;
                #ifdef USE_ADDR_COUNT_TABLE
                addr_count_table[offset].pos = npos + 2;
                #else
                addr_table[offset] = npos + 2;
                #endif
                return;
            }
            addr_slots[pos + 2] = chunk_id;
            addr_slots[pos + 3] = count;
            #ifdef USE_ADDR_COUNT_TABLE
            addr_count_table[offset].pos = pos + 2;
            #else
            addr_table[offset] = pos + 2;
            #endif
        }
    }
    uint32_t get_elapsed_ms() {
        return elapsed_ms;
    }
    inline static uint32_t offset_to_page(uint32_t offset) {
        return (offset >> ADDR_PAGE_BITS);
    }

    inline static void offset_info(uint32_t offset, uint32_t &page, uint32_t &addr, uint32_t thread_index) {
        page = offset >> ADDR_PAGE_BITS;
        uint32_t base_addr = page_to_addr(page);
        addr = ((offset & RELATIVE_OFFSET_MASK) << ADDR_LOW_BITS) + base_addr + thread_index * 8;
    }

    inline static uint32_t offset_to_addr(uint32_t offset, uint32_t thread_index) {
        uint32_t page = offset >> ADDR_PAGE_BITS;
        uint32_t base_addr = page_to_addr(page);
        return ((offset & RELATIVE_OFFSET_MASK) << ADDR_LOW_BITS) + base_addr + thread_index * 8;
    }

    inline static uint32_t addr_to_offset(uint32_t addr, uint32_t chunk_id = 0) {
        switch((uint8_t)((addr >> 24) & 0xFC)) {
            case 0x80: return ((addr - 0x80000000) >> (ADDR_LOW_BITS));
            case 0x84: return ((addr - 0x84000000) >> (ADDR_LOW_BITS)) + ADDR_PAGE_SIZE;
            case 0x90: return ((addr - 0x90000000) >> (ADDR_LOW_BITS)) + 2 * ADDR_PAGE_SIZE;
            case 0x94: return ((addr - 0x94000000) >> (ADDR_LOW_BITS)) + 3 * ADDR_PAGE_SIZE;
            case 0xA0: return ((addr - 0xA0000000) >> (ADDR_LOW_BITS)) + 4 * ADDR_PAGE_SIZE;
            case 0xA4: return ((addr - 0xA4000000) >> (ADDR_LOW_BITS)) + 5 * ADDR_PAGE_SIZE;
            case 0xA8: return ((addr - 0xA8000000) >> (ADDR_LOW_BITS)) + 6 * ADDR_PAGE_SIZE;
            case 0xAC: return ((addr - 0xAC000000) >> (ADDR_LOW_BITS)) + 7 * ADDR_PAGE_SIZE;
            case 0xB0: return ((addr - 0xB0000000) >> (ADDR_LOW_BITS)) + 8 * ADDR_PAGE_SIZE;
            case 0xB4: return ((addr - 0xB4000000) >> (ADDR_LOW_BITS)) + 9 * ADDR_PAGE_SIZE;
            case 0xB8: return ((addr - 0xB8000000) >> (ADDR_LOW_BITS)) + 10 * ADDR_PAGE_SIZE;
            case 0xBC: return ((addr - 0xBC000000) >> (ADDR_LOW_BITS)) + 11 * ADDR_PAGE_SIZE;
            case 0xC0: return ((addr - 0xC0000000) >> (ADDR_LOW_BITS)) + 12 * ADDR_PAGE_SIZE;
            case 0xC4: return ((addr - 0xC4000000) >> (ADDR_LOW_BITS)) + 13 * ADDR_PAGE_SIZE;
            case 0xC8: return ((addr - 0xC8000000) >> (ADDR_LOW_BITS)) + 14 * ADDR_PAGE_SIZE;
            case 0xCC: return ((addr - 0xCC000000) >> (ADDR_LOW_BITS)) + 15 * ADDR_PAGE_SIZE;
            case 0xD0: return ((addr - 0xD0000000) >> (ADDR_LOW_BITS)) + 16 * ADDR_PAGE_SIZE;
            case 0xD4: return ((addr - 0xD4000000) >> (ADDR_LOW_BITS)) + 17 * ADDR_PAGE_SIZE;
            case 0xD8: return ((addr - 0xD8000000) >> (ADDR_LOW_BITS)) + 18 * ADDR_PAGE_SIZE;
            case 0xDC: return ((addr - 0xDC000000) >> (ADDR_LOW_BITS)) + 19 * ADDR_PAGE_SIZE;
        }
        std::ostringstream msg;
        msg << "ERROR: addr_to_offset: 0x" << std::hex << addr << " (" << std::dec << chunk_id << ")";
        throw std::runtime_error(msg.str());
    }

    inline static uint32_t addr_to_page(uint32_t addr, uint32_t chunk_id = 0) {
        switch((uint8_t)((addr >> 24) & 0xFC)) {
            case 0x80: return 0;
            case 0x84: return 1;
            case 0x90: return 2;
            case 0x94: return 3;
            case 0xA0: return 4;
            case 0xA4: return 5;
            case 0xA8: return 6;
            case 0xAC: return 7;
            case 0xB0: return 8;
            case 0xB4: return 9;
            case 0xB8: return 10;
            case 0xBC: return 11;
            case 0xC0: return 12;
            case 0xC4: return 13;
            case 0xC8: return 14;
            case 0xCC: return 15;
            case 0xD0: return 16;
            case 0xD4: return 17;
            case 0xD8: return 18;
            case 0xDC: return 19;
        }
        std::ostringstream msg;
        msg << "ERROR: addr_to_page: 0x" << std::hex << addr << " (" << std::dec << chunk_id << ")";
        throw std::runtime_error(msg.str());
    }
    inline static uint32_t page_to_addr(uint8_t page) {
        switch(page) {
            case 0: return 0x80000000;
            case 1: return 0x84000000;
            case 2: return 0x90000000;
            case 3: return 0x94000000;
            case 4: return 0xA0000000;
            case 5: return 0xA4000000;
            case 6: return 0xA8000000;
            case 7: return 0xAC000000;
            case 8: return 0xB0000000;
            case 9: return 0xB4000000;
            case 10: return 0xB8000000;
            case 11: return 0xBC000000;
            case 12: return 0xC0000000;
            case 13: return 0xC4000000;
            case 14: return 0xC8000000;
            case 15: return 0xCC000000;
            case 16: return 0xD0000000;
            case 17: return 0xD4000000;
            case 18: return 0xD8000000;
            case 19: return 0xDC000000;
            case 0xFF: return 0xFFFFFFFF;
        }
        std::ostringstream msg;
        msg << "ERROR: MemCounter page_to_address page:" << page;
        throw std::runtime_error(msg.str());
    }
};
#endif