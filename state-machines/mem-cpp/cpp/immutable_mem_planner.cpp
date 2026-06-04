#include "immutable_mem_planner.hpp"
#include <vector>

ImmutableMemPlanner::ImmutableMemPlanner(uint32_t rows, uint32_t from_addr, uint32_t mb_size, bool intermediate_rows):
    rows_by_segment(rows),
    intermediate_rows(intermediate_rows),
    last_addr(0) {

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
    segments_pages.reserve(512);
    initial_last_addr = MemCounter::page_to_addr(from_page);
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
    
    // Initialize offset tracking with id=0 for immutable planner
    this->mb_size = mb_size;
    mem_offsets = new MemOffsets(from_page);
    mem_offsets->enable_debug(); // Enable debug for mem_offsets to track intermediate addresses    
}

ImmutableMemPlanner::~ImmutableMemPlanner() {
    delete current_segment;
    #ifndef MEM_CHECK_POINT_MAP
    delete hash_table;
    free(chunk_table);
    #endif
    delete mem_offsets;
}

// Calculate memory requirements for offset pages across all segments
// This function estimates the number of pages needed by identifying collapsible pages
// (pages with only one offset value), allowing us to:
// - Accurately predict memory consumption before allocation
// - Avoid running into memory limits during execution
// - Optimize storage by collapsing empty or single-value pages
void ImmutableMemPlanner::calculate_pages(const std::vector<MemCounter *> &workers) {
    uint32_t rows_available = rows_by_segment;
    uint32_t last_addr_with_data = 0;
    uint32_t segment_first_addr = 0;
    uint32_t offset, last_offset;
    uint32_t collapsible_pages = 0;

    uint32_t count = 0;
    for (uint32_t page = from_page; page <= to_page; ++page) {
        get_offset_limits(workers, page, offset, last_offset);
        for (; offset <= last_offset; ++offset) {
            for (uint32_t thread_index = 0; thread_index < MAX_THREADS; ++thread_index) {
                uint32_t real_count = workers[thread_index]->get_count_table(offset);
                if (real_count == 0) continue;
                
                uint32_t real_addr = MemCounter::offset_to_addr(offset, thread_index);
                
                // Process the real address and any intermediate addresses that precede it.
                // When intermediate_rows is enabled and the gap to the previous address exceeds
                // MAX_IMMUTABLE_ADDR_DISTANCE, we inject intermediate addresses (count=1 each)
                // to fill the gap, advancing MAX_IMMUTABLE_ADDR_DISTANCE at a time until we
                // reach real_addr. Intermediates are treated as regular addresses for row/segment
                // accounting purposes. The first address never generates intermediates
                // (last_addr_with_data == 0).
                uint32_t addr = 0;
                while (addr < real_addr) {
                    if (intermediate_rows && last_addr_with_data != 0 && (real_addr - last_addr_with_data) > MAX_IMMUTABLE_ADDR_DISTANCE) {
                        // Inject an intermediate address at MAX_IMMUTABLE_ADDR_DISTANCE from last data
                        addr = last_addr_with_data + MAX_IMMUTABLE_ADDR_DISTANCE;
                        count = 1;
                    } else {
                        // No intermediates needed: process the real address with its full count
                        addr = real_addr;
                        count = real_count;
                    }

                    // Process 'addr' (intermediate or real), handling segment boundaries:
                    // a single address with count > rows_available may span multiple segments.
                    while (count > 0) {
                        if (rows_available == 0) {
                            // Current segment is full: store its page/collapsible info and open a new one
                            if (segment_first_addr != 0) {
                                // Round up to nearest page
                                // + 8/8 - 1 = 0
                                uint32_t pages = (((last_addr_with_data - segment_first_addr + 1) / 8) + OFFSET_PAGE_SIZE) / OFFSET_PAGE_SIZE; 
                                segments_pages.push_back(std::make_pair(pages, collapsible_pages));
                                collapsible_pages = 0;
                            }
                            rows_available = rows_by_segment;
                            segment_first_addr = 0; // Will be set on first addr of new segment
                            last_addr_with_data = 0; // Reset: collapsible pages must not span segment boundaries
                        }
                        
                        // Record the first address of the current segment
                        if (segment_first_addr == 0) {                        
                            segment_first_addr = addr;
                        }
                        
                        // Calculate collapsible pages between last_addr_with_data and addr.
                        // A page is collapsible if it contains only one offset value:
                        // either it is fully within the gap (all slots filled with offset_value),
                        // or its last slot is 'addr' and all preceding slots are in the gap
                        // (all slots also have offset_value since it only increments after adding).
                        // Page p is collapsible iff: p >= last_data_page+1 AND p <= page_next_addr-1
                        // => collapsible count = page_next_addr - last_data_page - 1
                        if (last_addr_with_data != 0 && addr > (last_addr_with_data + 8)) {
                            // Distance in number of addresses (each address is 8 bytes apart)
                            uint32_t distance = (addr - last_addr_with_data) / 8;
                            
                            // Only relevant if the gap spans at least one full page
                            if ((distance + 1) >= OFFSET_PAGE_SIZE) {
                                // Index of last_addr_with_data within the segment
                                uint32_t index = (last_addr_with_data - segment_first_addr) / 8;
                                
                                // Page containing last_addr_with_data
                                uint32_t last_data_page = index / OFFSET_PAGE_SIZE;
                                
                                // Page of the slot immediately after addr
                                // (equals addr's page + 1 when addr is the last slot of its page)
                                uint32_t page_next_addr = (index + distance + 1) / OFFSET_PAGE_SIZE;
                                
                                // Collapsible pages are those entirely within [last_data_page+1, page_next_addr-1]
                                if (page_next_addr > last_data_page + 1) {
                                    uint32_t new_collapsible = page_next_addr - last_data_page - 1;
                                    uint32_t first_coll_page = last_data_page + 1;
                                    uint32_t last_coll_page  = page_next_addr - 1;
                                    uint32_t coll_addr_start = segment_first_addr + first_coll_page * OFFSET_PAGE_SIZE * 8;
                                    uint32_t coll_addr_end   = segment_first_addr + (last_coll_page + 1) * OFFSET_PAGE_SIZE * 8 - 8;
                                    collapsible_pages += new_collapsible;
                                }
                            }
                        }
                        
                        // Consume rows for this address (may be partial if near segment boundary)
                        uint32_t consumed = std::min(count, rows_available);
                        rows_available -= consumed;
                        count -= consumed;
                        
                        last_addr_with_data = addr;
                    }
                }
            }
        }
    }
    if (rows_available < rows_by_segment) {
        // + 8/8 - 1 = 0
        uint32_t pages = (((last_addr_with_data - segment_first_addr + 1) / 8) + OFFSET_PAGE_SIZE) / OFFSET_PAGE_SIZE;
        segments_pages.push_back(std::make_pair(pages, collapsible_pages));
    }
}

void ImmutableMemPlanner::execute(const std::vector<MemCounter *> &workers) {
    uint32_t addr = 0;
    uint32_t offset;
    uint32_t last_offset;
    calculate_pages(workers);
    init_mem_offsets();
    last_addr = initial_last_addr;
    for (uint32_t page = from_page; page <= to_page; ++page) {
        get_offset_limits(workers, page, offset, last_offset);
        addr = MemCounter::offset_to_addr(offset, 0);
        for (;offset <= last_offset; ++offset) {
            for (uint32_t i = 0; i < MAX_THREADS; ++i, addr += 8) {
                uint32_t pos = workers[i]->get_addr_table(offset);
                if (pos == 0) continue;
                uint32_t cpos = workers[i]->get_initial_pos(pos);
                while (cpos != 0) {
                    uint32_t chunk_id = workers[i]->get_pos_value(cpos);
                    uint32_t count = workers[i]->get_pos_count(cpos+1);
                    add_to_current_segment(chunk_id, addr, count);
                    if (cpos == pos) break;
                    cpos = workers[i]->get_next_pos(cpos+1);
                }
            }
        }
    }
    close_last_segment();
}

void ImmutableMemPlanner::get_offset_limits(const std::vector<MemCounter *> &workers, uint32_t page, uint32_t &first_offset, uint32_t &last_offset) {
    first_offset = workers[0]->first_offset[page];
    last_offset = workers[0]->last_offset[page];
    for (int i = 1; i < MAX_THREADS; ++i) {
        first_offset = std::min(first_offset, workers[i]->first_offset[page]);
        last_offset = std::max(last_offset, workers[i]->last_offset[page]);
    }
}

void ImmutableMemPlanner::close_segment() {
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
    #ifdef MEM_SAVE_CPP_OFFSETS
    mem_offsets->save_offsets_to_file("tmp/" + std::string(MemCounter::page_to_tag(from_page)) + "_trace_" + std::to_string((uint32_t)segments.size()) + "_cpp_offsets.txt", false);
    #endif
    
    // Move MemOffsets data to current_segment->offsets before saving
    mem_offsets->move_to_paged_offsets(current_segment->offsets, current_segment->offsets_base_addr);
    
    segments.emplace_back(current_segment);
    init_mem_offsets();
    
    #ifdef MEM_CHECK_POINT_MAP
    current_segment = new MemSegment();
    #else
    current_segment = new MemSegment(hash_table);
    #endif
}
void ImmutableMemPlanner::init_mem_offsets() {
    uint32_t next_segment_index = segments.size();
    if (next_segment_index < segments_pages.size()) {
        const auto pages_info = segments_pages[next_segment_index];
        mem_offsets->allocate(pages_info.first, pages_info.second); // Clear mem_offsets for next segment
    }
}
void ImmutableMemPlanner::open_segment() {
    #ifndef MEM_CHECK_POINT_MAP
    limit_pos = (segments.size() + 1) << 16;
    #endif
    close_segment();
    if (reference_addr_chunk != NO_CHUNK_ID) {
        #ifdef MEM_CHECK_POINT_MAP
        current_segment->add_or_update(reference_addr_chunk, reference_addr, reference_skip, 0);
        #else
        current_segment->add_or_update(hash_table, reference_addr_chunk, reference_addr, reference_skip, 0);
        #endif
        mem_offsets->update(reference_addr, reference_skip > 0 ? 0: 1);
    }
    rows_available = rows_by_segment;
}

void ImmutableMemPlanner::preopen_segment(uint32_t addr, uint32_t intermediate_rows) {
    if (rows_available == 0) {
        if (intermediate_rows > 0) {
            add_next_addr_to_segment(addr);
        }
        open_segment();
    }
}
void ImmutableMemPlanner::consume_rows(uint32_t addr, uint32_t row_count, uint32_t skip) {
    if (row_count == 0 && rows_available > 0) {
        return;
    }
    #ifdef DEBUG_MEM_CAP
    if (row_count > rows_available) {
        std::ostringstream msg;
        msg << "MemPlanner::consume " << row_count << " too much rows, available " << rows_available << std::endl;
        throw std::runtime_error(msg.str());
    }
    #endif
    if (rows_available == 0) {
        open_segment();
    }
    
    // Track offset before adding chunk
    mem_offsets->update(addr, rows_available == 0 && skip > 0 ? 0 : (rows_by_segment - rows_available + 1));
    
    add_chunk_to_segment(current_chunk, addr, row_count, skip);
    rows_available -= row_count;
    reference_skip += row_count;
}

void ImmutableMemPlanner::consume_intermediate_rows(uint32_t row_count) {
    if (row_count == 0 && rows_available > 0) {
        return;
    }
    #ifdef DEBUG_MEM_CAP
    if (row_count > rows_available) {
        std::ostringstream msg;
        msg << "MemPlanner::consume " << row_count << " too much rows, available " << rows_available << std::endl;
        throw std::runtime_error(msg.str());
    }
    #endif
    if (rows_available == 0) {
        open_segment();
        mem_offsets->reset();

    }
    // add_chunk_to_segment(current_chunk, addr, rows, skip);
    rows_available -= row_count;
}

void ImmutableMemPlanner::add_intermediate_rows(uint32_t count) {
    uint32_t pending = count;
    while (pending > 0) {
        uint32_t rows_consumed = std::min(pending, rows_available);
        consume_intermediate_rows(rows_consumed);
        pending -= rows_consumed;
    }
}

void ImmutableMemPlanner::add_rows(uint32_t addr, uint32_t count) {
    uint32_t pending = count;
    while (pending > 0) {
        uint32_t rows_consumed = std::min(pending, rows_available);
        uint32_t skip = count - pending;
        consume_rows(addr, rows_consumed, skip);
        pending -= rows_consumed;
    }
}

uint32_t ImmutableMemPlanner::add_intermediate_addr(uint32_t from_addr, uint32_t to_addr) {
    // adding internal reads of zero for consecutive addresses
    uint32_t count = (to_addr - from_addr + 8) >> 3;
    if (count > 1) {
        // add_intermediate_rows(from_addr, 1);
        // add_intermediate_rows(to_addr, count - 1);
        add_intermediate_rows(count);
    } else {
        // add_intermediate_rows(to_addr, 1);
        add_intermediate_rows(1);
    }
    return count;
}

uint32_t ImmutableMemPlanner::add_intermediates(uint32_t addr) {
    uint32_t count = 0;
    if (intermediate_rows && (addr - last_addr) > 8) {
        count = add_intermediate_addr(last_addr + 8, addr - 8);
    }
    last_addr = addr;
    return count;
}

void ImmutableMemPlanner::collect_segments(MemSegments &mem_segments) {
    uint32_t segment_id = 0;
    for (auto segment :segments) {
        mem_segments.set(segment_id++, segment);
    }
    segments.clear();
}

void ImmutableMemPlanner::stats() {

}

void ImmutableMemPlanner::save_offsets_to_file(const std::string &filename, bool compact) {
    mem_offsets->save_offsets_to_file(filename, compact);
}
