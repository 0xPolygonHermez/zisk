#ifndef __MEM_PLANNER_ORG_HPP__
#define __MEM_PLANNER_ORG_HPP__

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

#ifdef MEM_PLANNER_ORG
class MemPlannerOrg {
private:
    uint32_t rows;
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
    bool intermediate_step_reads;
    MemSegment *current_segment;
    std::vector<MemSegment *> segments;

public:
    MemPlannerOrg(uint32_t rows, uint32_t from_addr, uint32_t mb_size, bool intermediate_step_reads)
    :rows(rows), intermediate_step_reads(intermediate_step_reads) {
        rows_available = rows;
        reference_addr_chunk = NO_CHUNK_ID;
        reference_addr = 0;
        reference_skip = 0;
        current_chunk = NO_CHUNK_ID;
        #ifdef DIRECT_MEM_LOCATOR
        locators_count = 0;
        #endif
        current_segment = new MemSegment();
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
    ~MemPlannerOrg() {
    }
    void execute(const std::vector<MemCounter *> &workers) {
        uint32_t max_addr;
        uint32_t addr = 0;
        for (uint32_t page = from_page; page < to_page; ++page) {
            if (!(max_addr = get_max_addr(workers, page))) continue;
            addr = MemCounter::page_to_addr(page);
            uint32_t max_offset = MemCounter::addr_to_offset(max_addr);
            for (uint32_t offset = page * ADDR_PAGE_SIZE; offset <= max_offset; ++offset) {
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
        #ifdef SEGMENT_STATS
        printf("MemPlanner::execute  segments:%d tot_chunks:%d max_chunks:%d large_segments:%d avg_chunks:%04.2f\n", segments.size(), tot_chunks, max_chunks, large_segments, ((double)tot_chunks)/((double)(segments.size())));
        #endif
    }

    void execute_from_locators(const std::vector<MemCounter *> &workers, MemLocators &locators) {
        /*
        uint32_t max_addr;
        uint32_t addr = 0;
        MemLocator *plocator;
        uint32_t max_offset = 0;
        uint32_t offset = 0;
        bool completed = false;

        while (true) {
            plocator = locators.get_locator();
            if (plocator == nullptr) {
                if (completed && locators.is_completed()) {
                    break;
                }
                completed = true;
                usleep(1000);
                continue;
            }
            uint32_t skip = plocator->skip;
            uint32_t locator_page = MemCounter::offset_to_page(plocator->offset);
            bool first_loop = true;
            for (uint32_t page = locator_page; page < to_page; ++page) {
                if (first_loop) {
                    offset = plocator->offset;
                    MemCounter::offset_info(offset, page, addr);
                } else {
                    if (!(max_addr = get_max_addr(workers, page))) continue;
                    addr = MemCounter::page_to_addr(page);
                    max_offset = MemCounter::addr_to_offset(max_addr);
                    offset = page * ADDR_PAGE_SIZE;
                }
                for (;offset <= max_offset; ++offset) {
                    for (uint32_t i = 0; i < MAX_THREADS; ++i, addr += 8) {
                        uint32_t pos = workers[i]->get_addr_table(offset);
                        if (pos == 0) continue;
                        uint32_t cpos = workers[i]->get_initial_pos(pos);
                        while (cpos != 0) {
                            uint32_t chunk_id = workers[i]->get_pos_value(cpos);
                            uint32_t count = workers[i]->get_pos_value(cpos+1);
                            add_to_current_segment(chunk_id, addr, count, skip);
                            skip = 0;
                            if (cpos == pos) break;
                            cpos = workers[i]->get_next_pos(cpos+1);
                        }
                    }
                }
                first_loop = false;
            }
        }
        close_last_segment();
        #ifdef SEGMENT_STATS
        printf("MemPlanner::execute  segments:%d tot_chunks:%d max_chunks:%d large_segments:%d avg_chunks:%04.2f\n", segments.size(), tot_chunks, max_chunks, large_segments, ((double)tot_chunks)/((double)(segments.size())));
        #endif
        */
    }

    void generate_locators(const std::vector<MemCounter *> &workers, MemLocators &locators) {
        uint32_t max_addr;
        uint32_t addr = 0;
        rows_available = rows;
        uint32_t count;
        uint32_t p_chunk;
        for (uint32_t page = from_page; page < to_page; ++page) {
            if (!(max_addr = get_max_addr(workers, page))) continue;
            addr = MemCounter::page_to_addr(page);
            uint32_t max_offset = MemCounter::addr_to_offset(max_addr);
            for (uint32_t offset = page * ADDR_PAGE_SIZE; offset <= max_offset; ++offset) {
                for (uint32_t i = 0; i < MAX_THREADS; ++i, addr += 8) {
                    uint32_t pos = workers[i]->get_addr_table(offset);
                    if (pos == 0) continue;
                    uint32_t cpos = workers[i]->get_initial_pos(pos);
                    if (cpos == 0) continue;
                    count = workers[i]->get_pos_value(cpos+1);
                    if (rows_available <= count) {
                        locators.push_locator(offset, cpos, rows_available);
                        rows_available = rows_available + rows - count;
                    } else {
                        rows_available -= count;
                    }
                    if (cpos == pos) continue;
                    p_chunk = workers[i]->get_pos_value(cpos);
                    cpos = workers[i]->get_next_pos(cpos+1);
                    while (cpos != 0) {
                        uint32_t chunk = workers[i]->get_pos_value(cpos);
                        uint32_t count = workers[i]->get_pos_value(cpos+1);
                        uint32_t extra_row = (chunk - p_chunk) > CHUNK_MAX_DISTANCE ? 0:1;
                        count += extra_row;
                        if (rows_available > count) {
                            rows_available -= count;
                        } else if (rows_available < count) {
                            // TODO: add extra rows
                            locators.push_locator(offset, cpos, rows_available);
                            // skip = rows_available;
                            rows_available = rows_available + rows - count;
                        } else {
                            // TODO: add extra rows
                            locators.push_locator(offset, cpos, rows_available);
                            rows_available = rows;
                        }
                        if (cpos == pos) break;
                        p_chunk = chunk;
                        cpos = workers[i]->get_next_pos(cpos+1);
                    }
                }
            }
        }
/*
        #ifdef DIRECT_MEM_LOCATOR
        printf("MemPlanner::execute_fast: locators_count %d\n", locators_count);
        #else
        printf("MemPlanner::execute_fast: locators_count %d\n", locators.count);
        #endif

        #ifdef SEGMENT_STATS
        printf("MemPlanner::execute  segments:%d tot_chunks:%d max_chunks:%d large_segments:%d avg_chunks:%04.2f\n", segments.size(), tot_chunks, max_chunks, large_segments, ((double)tot_chunks)/((double)(segments.size())));
        #endif*/
    }
    uint32_t get_max_addr(const std::vector<MemCounter *> &workers, uint32_t page) {
        uint32_t max_addr = workers[0]->last_addr[page];
        for (int i = 1; i < MAX_THREADS; ++i) {
            if (workers[i]->last_addr[page] > max_addr) {
                max_addr = workers[i]->last_addr[page];
            }
        }
        return max_addr;
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
        if (rows_available < rows) {
            close_segment(true);
        } else if (segments.size() > 0) {
            segments.back()->is_last_segment = true;
        }
    }
    void close_segment(bool last = false) {
        current_segment->is_last_segment = last;
        // printf("MemPlanner::close_segment: %d chunks from_page:%d\n", current_segment->chunks.size(), from_page);
        #ifdef SEGMENT_STATS
        uint32_t segment_chunks = current_segment->chunks.size();
        if (segment_chunks > max_chunks) {
            max_chunks = segment_chunks;
        }
        if (segment_chunks >= SEGMENT_LARGE_CHUNKS) {
            large_segments++;
        }
        tot_chunks += segment_chunks;
        #endif

        segments.push_back(current_segment);
        current_segment = new MemSegment();
    }
    void open_segment(uint32_t intermediate_skip) {
        #ifndef MEM_CHECK_POINT_MAP
        limit_pos = (segments.size() + 1) << 16;
        #endif
        close_segment(false);
        if (reference_addr_chunk != NO_CHUNK_ID) {
            #ifdef MEM_CHECK_POINT_MAP
            current_segment->chunks.try_emplace(reference_addr_chunk, reference_addr, reference_skip, 0, intermediate_skip);
            #else
            current_segment->chunks.emplace_back(reference_addr_chunk, reference_addr, reference_skip, 0, intermediate_skip);
            #endif
        }
        rows_available = rows;
        // printf("MemPlanner::open_segment: rows_available: %d from_page:%d\n", rows_available, from_page);
    }
    void add_next_addr_to_segment(uint32_t addr) {
        add_chunk_to_segment(current_chunk, addr, 1, 0);
    }
    void add_chunk_to_segment(uint32_t chunk_id, uint32_t addr, uint32_t count, uint32_t skip) {
        #ifdef MEM_CHECK_POINT_MAP
        auto it = current_segment->chunks.find(chunk_id);
        if (it != current_segment->chunks.end()) {
            it->second.add_rows(addr, count);
        } else {
            current_segment->chunks.try_emplace(chunk_id, addr, skip, count, 0);
        }
        #else
        uint32_t pos = chunk_table[chunk_id];
        if (pos < limit_pos) {
            // not found
            // printf("MemPlanner::add_chunk_to_segment: chunk_id %d not found (pos: 0x%08X) size:%d limit_pos: 0x%08X\n", chunk_id, pos, current_segment->chunks.size(), limit_pos);
            chunk_table[chunk_id] = limit_pos + current_segment->chunks.size();
            current_segment->chunks.emplace_back(chunk_id, addr, skip, count, 0);
        } else {
            uint32_t vpos = pos & 0xFFFF;
            // printf("MemPlanner::add_chunk_to_segment: chunk_id %d already exists at vpos %d (pos: 0x%08X) size:%d limit_pos: 0x%08X\n", chunk_id, vpos, pos, current_segment->chunks.size(), limit_pos);
            current_segment->chunks[vpos].add_rows(addr, count);
        }
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

    void consume_intermediate_rows(uint32_t addr, uint32_t row_count, uint32_t skip) {
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
        if (intermediate_step_reads) {
            add_chunk_to_segment(current_chunk, addr, rows, skip);
        }
        rows_available -= row_count;
    }

    void add_intermediate_rows(uint32_t addr, uint32_t count) {
        uint32_t pending = count;
        while (pending > 0) {
            uint32_t rows = std::min(pending, rows_available);
            uint32_t skip = count - pending;
            consume_intermediate_rows(addr, rows, skip);
            pending -= rows;
        }
    }

    void add_rows(uint32_t addr, uint32_t count) {
        uint32_t pending = count;
        while (pending > 0) {
            uint32_t rows = std::min(pending, rows_available);
            uint32_t skip = count - pending;
            consume_rows(addr, rows, skip);
            pending -= rows;
        }
    }

    void add_intermediate_addr(uint32_t from_addr, uint32_t to_addr) {
        // adding internal reads of zero for consecutive addresses
        uint32_t count = to_addr - from_addr + 1;
        if (count > 1) {
            add_intermediate_rows(from_addr, 1);
            add_intermediate_rows(to_addr, count - 1);
        } else {
            add_intermediate_rows(to_addr, 1);
        }
    }

    uint32_t add_intermediates(uint32_t addr) {
        if (last_addr != addr) {
            if (!intermediate_step_reads && (addr - last_addr) > 1) {
                add_intermediate_addr(last_addr + 1, addr - 1);
            }
            last_addr = addr;
        } else if (intermediate_step_reads) {
            return add_intermediate_steps(addr);
        }
        return 0;
    }

    uint32_t add_intermediate_steps(uint32_t addr) {
        // check if the distance between the last chunk and the current is too large,
        // if so then we need to add intermediate rows
        uint32_t intermediate_rows = 0;
        if (reference_addr_chunk != NO_CHUNK_ID) {
            uint32_t chunk_distance = current_chunk - reference_addr_chunk;
            if (chunk_distance > CHUNK_MAX_DISTANCE) {
                this->add_intermediate_rows(addr, 1);
            }
        }
        return intermediate_rows;
    }
    void stats() {

    }
};
#endif
#endif