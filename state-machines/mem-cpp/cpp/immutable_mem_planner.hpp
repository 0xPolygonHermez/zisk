#ifndef __INMUTABLE_MEM_PLANNER_HPP__
#define __INMUTABLE_MEM_PLANNER_HPP__

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <unistd.h>
#include <thread>
#include <iostream>
#include <string.h>
#include <sys/time.h>
#include <cstdint>
#include <vector>
#include <map>
#include <unordered_map>
#include <stdexcept>
#include <mutex>
#include <atomic>

#include "mem_types.hpp"
#include "mem_config.hpp"
#include "tools.hpp"
#include "mem_counter.hpp"
#include "mem_segment.hpp"
#include "mem_check_point.hpp"
#include "mem_locators.hpp"
#include "mem_locator.hpp"
#include "mem_segments.hpp"

class ImmutableMemPlanner {
private:
    uint32_t rows_by_segment;
    uint32_t from_page;
    uint32_t to_page;
    uint32_t rows_available;
    uint32_t reference_addr_chunk;
    uint32_t reference_addr;
    uint32_t reference_skip;
    uint32_t current_chunk;
    uint32_t last_addr;
    #ifndef MEM_CHECK_POINT_MAP
    uint32_t *chunk_table;
    uint32_t limit_pos;
    #endif
    #ifdef SEGMENT_STATS
    uint32_t max_chunks;
    uint32_t large_segments;
    uint32_t tot_chunks;
    #endif
    #ifdef DIRECT_MEM_LOCATOR
    MemLocator locators[MAX_CHUNKS];
    uint32_t locators_count;
    #endif
    MemSegment *current_segment;
    #ifndef MEM_CHECK_POINT_MAP
    MemSegmentHashTable *hash_table;
    #endif
    std::vector<MemSegment *> segments;

public:
    ImmutableMemPlanner(uint32_t rows, uint32_t from_addr, uint32_t mb_size):rows_by_segment(rows) {
        #ifndef MEM_CHECK_POINT_MAP
        hash_table = new MemSegmentHashTable(MAX_CHUNKS);   // 2^18 * 2^18 = 2^36   // 2^14 * 2^18 = 2^32
        #endif
        rows_available = rows;
        reference_addr_chunk = NO_CHUNK_ID;
        reference_addr = 0;
        reference_skip = 0;
        current_chunk = NO_CHUNK_ID;
        #ifdef DIRECT_MEM_LOCATOR
        locators_count = 0;
        #endif
        #ifdef MEM_CHECK_POINT_MAP
        current_segment = new MemSegment();
        #else
        current_segment = new MemSegment(hash_table);
        #endif
        from_page = MemCounter::addr_to_page(from_addr);
        to_page = MemCounter::addr_to_page(from_addr + (mb_size * 1024 * 1024) - 1);
        if (MemCounter::page_to_addr(from_page) != from_addr) {
            std::ostringstream msg;
            msg << "MemPlanner::constructor: from_addr " << std::hex << from_addr << " not aligned to page " << std::dec << from_page;
            throw std::runtime_error(msg.str());
        }
        #ifndef MEM_CHECK_POINT_MAP
        chunk_table = (uint32_t *)malloc(MAX_CHUNKS * sizeof(uint32_t));
        memset(chunk_table, 0, MAX_CHUNKS * sizeof(uint32_t));
        limit_pos = 0x00010000;
        #endif
        #ifdef SEGMENT_STATS
        max_chunks = 0;
        tot_chunks = 0;
        large_segments = 0;
        #endif
    }
    ~ImmutableMemPlanner() {
    }
    void execute(const std::vector<MemCounter *> &workers) {
        uint32_t max_addr;
        uint32_t addr = 0;
        uint32_t offset;
        uint32_t last_offset;
        last_addr = MemCounter::page_to_addr(from_page);
        for (uint32_t page = from_page; page < to_page; ++page) {
            get_offset_limits(workers, page, offset, last_offset);
            // printf("##### page:%d offsets:0x%08X-0x%08X\n", page, offset, last_offset);
            addr = MemCounter::offset_to_addr(offset, 0);
            for (;offset <= last_offset; ++offset) {
                // printf("offset:0x%08X page:%d addr:0x%08X segments:%d\n", offset, page, addr, segments.size());
                for (uint32_t i = 0; i < MAX_THREADS; ++i, addr += 8) {
                    uint32_t pos = workers[i]->get_addr_table(offset);
                    if (pos == 0) continue;
                    uint32_t cpos = workers[i]->get_initial_pos(pos);
                    while (cpos != 0) {
                        uint32_t chunk_id = workers[i]->get_pos_value(cpos);
                        uint32_t count = workers[i]->get_pos_value(cpos+1);
                        add_to_current_segment(chunk_id, addr, count);
                        if (cpos == pos) break;
                        cpos = workers[i]->get_next_pos(cpos+1);
                    }
                }
            }
        }
        close_last_segment();
    }

    void get_offset_limits(const std::vector<MemCounter *> &workers, uint32_t page, uint32_t &first_offset, uint32_t &last_offset) {
        first_offset = workers[0]->first_offset[page];
        last_offset = workers[0]->last_offset[page];
        for (int i = 1; i < MAX_THREADS; ++i) {
            first_offset = std::min(first_offset, workers[i]->first_offset[page]);
            last_offset = std::max(last_offset, workers[i]->last_offset[page]);
        }
    }

    void add_to_current_segment(uint32_t chunk_id, uint32_t addr, uint32_t count) {
        set_current_chunk(chunk_id);
        uint32_t intermediate_rows = add_intermediates(addr);
        preopen_segment(addr, intermediate_rows);
        set_reference(chunk_id, addr);
        add_rows(addr, count);
    }
    void set_reference(uint32_t chunk_id, uint32_t addr) {
        reference_addr_chunk = chunk_id;
        reference_addr = addr;
        reference_skip = 0;
    }
    void set_current_chunk(uint32_t chunk_id) {
        current_chunk = chunk_id;
    }
    void close_last_segment() {
        if (rows_available < rows_by_segment) {
            close_segment(true);
        }/* else if (segments.size() > 0) {
            segments.back()->is_last_segment = true;
        }*/
    }
    void close_segment(bool last = false) {
        // current_segment->is_last_segment = last;
        // printf("MemPlanner::close_segment: %d chunks from_page:%d\n", current_segment->chunks.size(), from_page);
        #ifdef SEGMENT_STATS
        uint32_t segment_chunks = current_segment->size();
        if (segment_chunks > max_chunks) {
            max_chunks = segment_chunks;
        }
        if (segment_chunks >= SEGMENT_LARGE_CHUNKS) {
            large_segments++;
        }
        tot_chunks += segment_chunks;
        #endif

        segments.emplace_back(current_segment);
        #ifdef MEM_CHECK_POINT_MAP
        current_segment = new MemSegment();
        #else
        current_segment = new MemSegment(hash_table);
        #endif
    }
    void open_segment(uint32_t intermediate_skip) {
        #ifndef MEM_CHECK_POINT_MAP
        limit_pos = (segments.size() + 1) << 16;
        #endif
        close_segment(false);
        if (reference_addr_chunk != NO_CHUNK_ID) {
            #ifdef MEM_CHECK_POINT_MAP
            current_segment->add_or_update(reference_addr_chunk, reference_addr, reference_skip, 0);
            #else
            current_segment->add_or_update(hash_table, reference_addr_chunk, reference_addr, reference_skip, 0);
            #endif
        }
        rows_available = rows_by_segment;
        // printf("MemPlanner::open_segment: rows_available: %d from_page:%d\n", rows_available, from_page);
    }
    void add_next_addr_to_segment(uint32_t addr) {
        add_chunk_to_segment(current_chunk, addr, 1, 0);
    }
    void add_chunk_to_segment(uint32_t chunk_id, uint32_t addr, uint32_t count, uint32_t skip) {
            #ifdef MEM_CHECK_POINT_MAP
        current_segment->add_or_update(chunk_id, addr, skip, count);
            #else
        current_segment->add_or_update(hash_table, chunk_id, addr, skip, count);
            #endif
    }
    void preopen_segment(uint32_t addr, uint32_t intermediate_rows) {
        if (rows_available == 0) {
            if (intermediate_rows > 0) {
                add_next_addr_to_segment(addr);
            }
            open_segment(intermediate_rows);
        }
    }
    void consume_rows(uint32_t addr, uint32_t row_count, uint32_t skip) {
        if (row_count == 0 && rows_available > 0) {
            return;
        }
        if (row_count > rows_available) {
            std::ostringstream msg;
            msg << "MemPlanner::consume " << row_count << " too much rows, available " << rows_available << std::endl;
            throw std::runtime_error(msg.str());
        }
        if (rows_available == 0) {
            open_segment(0);
        }
        add_chunk_to_segment(current_chunk, addr, row_count, skip);
        rows_available -= row_count;
        reference_skip += row_count;
    }

    void consume_intermediate_rows(uint32_t addr, uint32_t row_count) {
        if (row_count == 0 && rows_available > 0) {
            return;
        }
        if (row_count > rows_available) {
            std::ostringstream msg;
            msg << "MemPlanner::consume " << row_count << " too much rows, available " << rows_available << std::endl;
            throw std::runtime_error(msg.str());
        }
        if (rows_available == 0) {
            open_segment(0);
        }
        // TODO: REVIEW
        // add_chunk_to_segment(current_chunk, addr, rows, skip);
        rows_available -= row_count;
    }

    void add_intermediate_rows(uint32_t addr, uint32_t count) {
        uint32_t pending = count;
        while (pending > 0) {
            uint32_t rows_consumed = std::min(pending, rows_available);
            consume_intermediate_rows(addr, rows_consumed);
            pending -= rows_consumed;
        }
    }

    void add_rows(uint32_t addr, uint32_t count) {
        uint32_t pending = count;
        while (pending > 0) {
            uint32_t rows_consumed = std::min(pending, rows_available);
            uint32_t skip = count - pending;
            consume_rows(addr, rows_consumed, skip);
            pending -= rows_consumed;
        }
    }

    uint32_t add_intermediate_addr(uint32_t from_addr, uint32_t to_addr) {
        // adding internal reads of zero for consecutive addresses
        uint32_t count = (to_addr - from_addr + 8) >> 3;
        if (count > 1) {
            add_intermediate_rows(from_addr, 1);
            add_intermediate_rows(to_addr, count - 1);
        } else {
            add_intermediate_rows(to_addr, 1);
        }
        return count;
    }

    uint32_t add_intermediates(uint32_t addr) {
        uint32_t count = 0;
        if ((addr - last_addr) > 8) {
            count = add_intermediate_addr(last_addr + 8, addr - 8);
        }
        last_addr = addr;
        return count;
    }

    void collect_segments(MemSegments &mem_segments) {
        uint32_t segment_id = 0;
        for (auto segment :segments) {
            mem_segments.set(segment_id++, segment);
        }
        segments.clear();
    }
    void stats() {

    }
};
#endif