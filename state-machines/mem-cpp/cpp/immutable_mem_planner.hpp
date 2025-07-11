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
    ImmutableMemPlanner(uint32_t rows, uint32_t from_addr, uint32_t mb_size);
    ~ImmutableMemPlanner();
    void execute(const std::vector<MemCounter *> &workers);
    void get_offset_limits(const std::vector<MemCounter *> &workers, uint32_t page, uint32_t &first_offset, uint32_t &last_offset);
    inline void add_to_current_segment(uint32_t chunk_id, uint32_t addr, uint32_t count);
    inline void set_reference(uint32_t chunk_id, uint32_t addr);
    inline void set_current_chunk(uint32_t chunk_id);
    inline void close_last_segment();
    void close_segment();
    void open_segment();
    inline void add_next_addr_to_segment(uint32_t addr);
    inline void add_chunk_to_segment(uint32_t chunk_id, uint32_t addr, uint32_t count, uint32_t skip);
    void preopen_segment(uint32_t addr, uint32_t intermediate_rows);
    void consume_rows(uint32_t addr, uint32_t row_count, uint32_t skip);
    void consume_intermediate_rows(uint32_t row_count);
    void add_intermediate_rows(uint32_t count);
    void add_rows(uint32_t addr, uint32_t count);
    uint32_t add_intermediate_addr(uint32_t from_addr, uint32_t to_addr);
    uint32_t add_intermediates(uint32_t addr);
    void collect_segments(MemSegments &mem_segments);
    void stats();
};

void ImmutableMemPlanner::add_to_current_segment(uint32_t chunk_id, uint32_t addr, uint32_t count) {
    set_current_chunk(chunk_id);
    uint32_t intermediate_rows = add_intermediates(addr);
    preopen_segment(addr, intermediate_rows);
    set_reference(chunk_id, addr);
    add_rows(addr, count);
}

void ImmutableMemPlanner::set_reference(uint32_t chunk_id, uint32_t addr) {
    reference_addr_chunk = chunk_id;
    reference_addr = addr;
    reference_skip = 0;
}

void ImmutableMemPlanner::set_current_chunk(uint32_t chunk_id) {
    current_chunk = chunk_id;
}

void ImmutableMemPlanner::close_last_segment() {
    if (rows_available < rows_by_segment) {
        close_segment();
    }/* else if (segments.size() > 0) {
        segments.back()->is_last_segment = true;
    }*/
}

void ImmutableMemPlanner::add_next_addr_to_segment(uint32_t addr) {
    add_chunk_to_segment(current_chunk, addr, 1, 0);
}

void ImmutableMemPlanner::add_chunk_to_segment(uint32_t chunk_id, uint32_t addr, uint32_t count, uint32_t skip) {
        #ifdef MEM_CHECK_POINT_MAP
    current_segment->add_or_update(chunk_id, addr, skip, count);
        #else
    current_segment->add_or_update(hash_table, chunk_id, addr, skip, count);
        #endif
}

#endif
