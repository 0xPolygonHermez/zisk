#include "mem_planner.hpp"

MemPlanner::MemPlanner(uint32_t id, uint32_t rows, uint32_t from_addr, uint32_t mb_size)
#ifdef MEM_CHECK_POINT_MAP
:id(id),rows(rows) {
#else
:id(id),rows(rows), hash_table(MAX_CHUNKS) {
#endif
    rows_available = rows;
    reference_addr_chunk = NO_CHUNK_ID;
    reference_addr = 0;
    reference_skip = 0;
    locators_done = 0;
    #ifdef MEM_PLANNER_STATS
    locators_time_count = 0;
    #endif

    current_chunk = NO_CHUNK_ID;
    #ifdef DIRECT_MEM_LOCATOR
    locators_count = 0;
    #endif
    current_segment = nullptr;
    from_page = MemCounter::addr_to_page(from_addr);
    to_page = MemCounter::addr_to_page(from_addr + (mb_size * 1024 * 1024) - 1);
    #ifdef MEM_DEBUG
    if (MemCounter::page_to_addr(from_page) != from_addr) {
        std::ostringstream msg;
        msg << "MemPlanner::constructor: from_addr " << std::hex << from_addr << " not aligned to page " << std::dec << from_page;
        throw std::runtime_error(msg.str());
    }
    #endif
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

MemPlanner::~MemPlanner() {
    if (current_segment) delete current_segment;
    #ifndef MEM_CHECK_POINT_MAP
    free(chunk_table);
    #endif
}

const MemLocator *MemPlanner::get_next_locator(MemLocators &locators, uint32_t &segment_id, uint32_t us_timeout) {
    const MemLocator *plocator = locators.get_locator(segment_id);
    bool completed = false;
    while (plocator == nullptr) {
        if (completed && locators.is_completed()) {
            return nullptr;
        }
        plocator = locators.get_locator(segment_id);
        if (plocator != nullptr) {
            return plocator;
        }
        completed = true;
        usleep(us_timeout);
        continue;
    }
    return plocator;
}

void MemPlanner::execute_from_locators(const std::vector<MemCounter *> &workers, MemLocators &locators, MemSegments &segments) {
    uint64_t init = get_usec();
    const MemLocator *locator;
    uint32_t segment_id = 0;
    while (true) {
        if ((locator = get_next_locator(locators, segment_id)) == nullptr) {
            break;
        }
        execute_from_locator(workers, segment_id, locator);
        segments.set(segment_id, current_segment);
        current_segment = nullptr;
    }
    elapsed = get_usec() - init;
}

void MemPlanner::execute_from_locator(const std::vector<MemCounter *> &workers, uint32_t segment_id, const MemLocator *locator) {
    uint32_t addr = 0;

    ++locators_done;
    #ifdef MEM_PLANNER_STATS
    uint32_t addr_count = 0;
    uint32_t offset_count = 0;
    #endif
    uint32_t skip = locator->skip;
    uint32_t offset = locator->offset;
    uint32_t page = MemCounter::offset_to_page(offset);
    uint32_t max_offset = get_max_offset(workers, page);
    uint32_t thread_index = locator->thread_index;
    uint32_t cpos = locator->cpos;
    bool first_pos = true;
    #ifdef MEM_PLANNER_STATS
    uint32_t first_segment_addr = MemCounter::offset_to_addr(offset, thread_index);
    uint32_t last_segment_addr = first_segment_addr;
    #endif
    for (;page < to_page; ++page, thread_index = 0, get_offset_limits(workers, page, offset, max_offset)) {
        for (;offset <= max_offset; ++offset, thread_index = 0) {
            addr = MemCounter::offset_to_addr(offset, thread_index);
            #ifdef MEM_PLANNER_STATS
            ++offset_count;
            #endif
            for (;thread_index < MAX_THREADS; ++thread_index, addr += 8, first_pos = false) {
                uint32_t pos = workers[thread_index]->get_addr_table(offset);
                if (pos == 0) {
                    if (first_pos) printf("************ ERROR SEGMENT %d thread_index %d offset %d addr 0x%08X\n", segment_id, thread_index, offset, addr);
                    continue;
                }
                #ifdef MEM_PLANNER_STATS
                last_segment_addr = addr;
                ++addr_count;
                #endif
                if (segment_id == 0 || first_pos == false) {          
                    skip = 0;      
                    cpos = workers[thread_index]->get_initial_pos(pos); 
                }
                while (cpos != 0) {
                    uint32_t chunk_id = workers[thread_index]->get_pos_value(cpos);
                    uint32_t count = workers[thread_index]->get_pos_value(cpos+1);
                    if (add_chunk(chunk_id, addr, count - skip, skip) == false) {
                        #ifdef MEM_PLANNER_STATS
                        update_segment_stats(addr_count, offset_count, first_segment_addr, last_segment_addr);
                        #endif
                        return;
                    }
                    skip = 0;
                    if (cpos == pos) break;
                    cpos = workers[thread_index]->get_next_pos(cpos+1);
                }
            }
        }
    }
    #ifdef MEM_PLANNER_STATS
    update_segment_stats(addr_count, offset_count, first_segment_addr, last_segment_addr);
    #endif
}

#ifdef MEM_PLANNER_STATS
void MemPlanner::update_segment_stats(uint32_t addr_count, uint32_t offset_count, uint32_t first_segment_addr, uint32_t last_segment_addr) {
    uint32_t index = segments.size();
    segment_stats[index].addr_count = addr_count;
    segment_stats[index].offset_count = offset_count;
    segment_stats[index].first_addr = first_segment_addr;
    segment_stats[index].last_addr = last_segment_addr;
    segment_stats[index].chunks = current_segment->size();
}
#endif

void MemPlanner::generate_locators(const std::vector<MemCounter *> &workers, MemLocators &locators) {
    uint64_t init = get_usec();
    rows_available = rows;
    uint32_t count;
    uint32_t offset, max_offset;
    bool inserted_first_locator = false;
    for (uint32_t page = from_page; page < to_page; ++page) {
        get_offset_limits(workers, page, offset, max_offset);
        for (;offset <= max_offset; ++offset) {
            for (uint32_t thread_index = 0; thread_index < MAX_THREADS; ++thread_index) {
                uint32_t pos = workers[thread_index]->get_addr_table(offset);
                if (pos == 0) continue;
                if (inserted_first_locator == false) {
                    inserted_first_locator = true;
                    locators.push_locator(thread_index, offset, pos, 0);
                }
                uint32_t addr_count = workers[thread_index]->get_count_table(offset);
                if (rows_available > addr_count) {
                    rows_available -= addr_count;
                    continue;
                }
                uint32_t cpos = workers[thread_index]->get_initial_pos(pos);
                while (true) {
                    count = workers[thread_index]->get_pos_value(cpos+1);
                    uint32_t initial_count = count;
                    while (count > 0) {
                        if (rows_available > count) {
                            rows_available -= count;
                            break;
                        } else if (rows_available <= count) {
                            // when rows_available == count, we need to pass by offset,cpos to get last value
                            #ifdef MEM_PLANNER_STATS
                            if (locators_time_count < 8) {
                                locators_times[locators_time_count++] = get_usec() - init;
                            }
                            #endif
                            count -= rows_available;
                            uint32_t skip = initial_count - count;
                            locators.push_locator(thread_index, offset, cpos, skip);
                            rows_available = rows;
                        }
                    }
                    if (pos == cpos) break;
                    cpos = workers[thread_index]->get_next_pos(cpos+1);
                }
            }
        }
    }
    locators.set_completed();
    elapsed = get_usec() - init;
}

void MemPlanner::get_offset_limits(const std::vector<MemCounter *> &workers, uint32_t page, uint32_t &first_offset, uint32_t &last_offset) {
    first_offset = workers[0]->first_offset[page];
    last_offset = workers[0]->last_offset[page];
    for (int i = 1; i < MAX_THREADS; ++i) {
        first_offset = std::min(first_offset, workers[i]->first_offset[page]);
        last_offset = std::max(last_offset, workers[i]->last_offset[page]);
    }
}

uint32_t MemPlanner::get_max_offset(const std::vector<MemCounter *> &workers, uint32_t page) {
    uint32_t last_offset = workers[0]->last_offset[page];
    for (int i = 1; i < MAX_THREADS; ++i) {
        last_offset = std::max(last_offset, workers[i]->last_offset[page]);
    }
    return last_offset;
}

bool MemPlanner::add_chunk(uint32_t chunk_id, uint32_t addr, uint32_t count, uint32_t skip) {
    if (current_segment == nullptr) {
        // include first chunk
        uint32_t consumed = std::min(count, rows);
        #ifdef MEM_CHECK_POINT_MAP
        current_segment = new MemSegment(chunk_id, addr, skip, consumed);
        #else
        current_segment = new MemSegment(hash_table, chunk_id, addr, skip, consumed);
        #endif
        rows_available = rows - consumed;
        return (rows_available != 0);
    }
    if (rows_available <= count) {
        current_segment_add(chunk_id, addr, rows_available);
        rows_available = 0;
        return false;
    }
    current_segment_add(chunk_id, addr, count);
    rows_available -= count;
    return true;
}

void MemPlanner::current_segment_add(uint32_t chunk_id, uint32_t addr, uint32_t count) {
    #ifdef MEM_CHECK_POINT_MAP
    current_segment->add_or_update(chunk_id, addr, 0, count);
    #else
    current_segment->add_or_update(hash_table, chunk_id, addr, 0, count);
    #endif
}

void MemPlanner::stats() {
    printf("PLANNER|I: %2d|D: %4d|%7.2f ms\n", id, locators_done, elapsed / 1000.0);
    #ifdef MEM_PLANNER_STATS
    for (uint32_t index = 0; index < locators_time_count; ++index) {
        printf("MemPlanner::stats: locators_time[%d]: %lu\n", index, locators_times[index]);
    }
    uint32_t count = segments.size();
    for (uint32_t index = 0; index < count; ++index) {
        printf("SEGMENT_STAT|0x%08X-0x%08X|T: %2d|S: %3d|C: %4d|@: %7d|O: %7d\n",
            segment_stats[index].first_addr,
            segment_stats[index].last_addr,
            id,
            index,
            segments[index]->size(),
            segment_stats[index].addr_count,
            segment_stats[index].offset_count);
    }
    #endif
}
