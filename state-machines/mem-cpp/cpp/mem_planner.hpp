#ifndef __MEM_PLANNER_HPP__
#define __MEM_PLANNER_HPP__

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

#ifdef MEM_PLANNER_STATS
struct SegmentStats {
    uint32_t addr_count;
    uint32_t offset_count;
    uint32_t first_addr;
    uint32_t last_addr;
    uint32_t chunks;
};
#define MAX_SEGMENTS 1024
#endif
class MemPlanner {
private:
    uint32_t id;
    uint32_t rows;
    uint32_t from_page;
    uint32_t to_page;
    uint32_t rows_available;
    uint32_t reference_addr_chunk;
    uint32_t reference_addr;
    uint32_t reference_skip;
    uint32_t current_chunk;
    uint32_t last_addr;
    uint32_t locators_done;
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
    #ifdef MEM_PLANNER_STATS
    uint64_t locators_times[8];
    uint32_t locators_time_count;
    SegmentStats segment_stats[MAX_SEGMENTS];
    #endif
    uint64_t elapsed;
    #ifndef MEM_CHECK_POINT_MAP
    MemSegmentHashTable hash_table;
    #endif

public:
    MemPlanner(const MemPlanner&) = delete;
    MemPlanner& operator=(const MemPlanner&) = delete;
    MemPlanner(MemPlanner&&) noexcept = default;
    
    MemPlanner(uint32_t id, uint32_t rows, uint32_t from_addr, uint32_t mb_size);
    ~MemPlanner();
    const MemLocator *get_next_locator(MemLocators &locators, uint32_t &segment_id, uint32_t us_timeout = 10);
    void execute_from_locators(const std::vector<MemCounter *> &workers, MemLocators &locators, MemSegments &segments);        
    void execute_from_locator(const std::vector<MemCounter *> &workers, uint32_t segment_id, const MemLocator *locator);
    #ifdef MEM_PLANNER_STATS
    void update_segment_stats(uint32_t addr_count, uint32_t offset_count, uint32_t first_segment_addr, uint32_t last_segment_addr);
    #endif
    void generate_locators(const std::vector<MemCounter *> &workers, MemLocators &locators);
    void get_offset_limits(const std::vector<MemCounter *> &workers, uint32_t page, uint32_t &first_offset, uint32_t &last_offset);
    uint32_t get_max_offset(const std::vector<MemCounter *> &workers, uint32_t page);
    bool add_chunk(uint32_t chunk_id, uint32_t addr, uint32_t count, uint32_t skip = 0);
    void current_segment_add(uint32_t chunk_id, uint32_t addr, uint32_t count);
    void stats();
    inline uint64_t *get_locators_times(uint32_t &count);
};


uint64_t *MemPlanner::get_locators_times(uint32_t &count) {
    #ifdef MEM_PLANNER_STATS
    count = locators_time_count;
    return locators_times;
    #else
    count = 0;
    return nullptr;
    #endif
}
#endif
